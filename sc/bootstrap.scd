// Index v1 headless sequencing bridge. No audio server is used.
// Invocation: sclang -D bootstrap.scd <Rust UDP port>
var rustPort = "INDEX_RUST_PORT".getenv.asInteger;
var scPort = "INDEX_SC_PORT".getenv.asInteger;
var handle;
var start;
var rust = NetAddr("127.0.0.1", rustPort);
var clock = TempoClock(2.0);
var config = (name: "bass", degrees: [0], durations: [1.0], velocities: [100], channel: 1, destination: "Monitor");
var playing = false;
var runner;
var emit = { |kind, note, velocity, channel, destination|
    rust.sendMsg("/index/v1/event/midi", Main.elapsedTime, kind, note, velocity, channel, destination);
};
var parseInts = { |x| x.split($,).collect(_.asInteger) };
var parseFloats = { |x| x.split($,).collect(_.asFloat) };
var stop = { playing = false; if(runner.notNil) { runner.stop; runner = nil }; rust.sendMsg("/index/v1/state", 0) };
start = {
    if(playing.not) {
        playing = true; rust.sendMsg("/index/v1/state", 1);
        runner = Routine({ var stream, ev, i = 0, dur, note;
            stream = Pbindef(config.name.asSymbol,
                \degree, Pseq(config.degrees, inf),
                \dur, Pseq(config.durations, inf),
                \amp, Pseq(config.velocities.collect({ |v| v / 127 }), inf),
                \midichan, config.channel - 1).asStream;
            while { playing } {
                ev = stream.next(()); i = i % config.degrees.size;
                dur = ev.dur; note = (60 + config.degrees[i]).clip(0, 127);
                emit.("on", note, config.velocities[i], config.channel, config.destination);
                { dur.wait; emit.("off", note, 0, config.channel, config.destination) }.fork;
                i = i + 1; dur.wait;
            }
        }).play(clock);
    }
};
handle = { |msg|
    var path = msg[0].asString;
    try {
        switch(path,
            "/index/v1/hello", { rust.sendMsg("/index/v1/ready", 1, "bass", 120.0) },
            "/index/v1/transport/play", { start.() },
            "/index/v1/transport/stop", { stop.() },
            "/index/v1/tempo", { clock.tempo = msg[1].asFloat / 60; rust.sendMsg("/index/v1/state", playing.asInteger) },
            "/index/v1/pattern/set", { config = (name: msg[1].asString, degrees: parseInts.(msg[2].asString), durations: parseFloats.(msg[3].asString), velocities: parseInts.(msg[4].asString), channel: msg[5].asInteger, destination: msg[6].asString); rust.sendMsg("/index/v1/state", playing.asInteger) },
            "/index/v1/pattern/commit", { if(msg[1].asInteger > 0) { clock.schedAbs(clock.nextBar, { if(playing) { stop.(); start.() }; nil }) } { if(playing) { stop.(); start.() } } },
            "/index/v1/code/eval", { msg[1].asString.interpret; },
            { rust.sendMsg("/index/v1/error", "unknown OSC path: " ++ path) }
        );
    } { |err| rust.sendMsg("/index/v1/error", err.errorString) };
};
[
    "/index/v1/hello", "/index/v1/transport/play", "/index/v1/transport/stop",
    "/index/v1/tempo", "/index/v1/pattern/set", "/index/v1/pattern/commit",
    "/index/v1/code/eval"
].do({ |path| OSCFunc(handle, path, recvPort: scPort) });
rust.sendMsg("/index/v1/ready", 1, "bass", 120.0);
fork { loop { 1.wait } };
