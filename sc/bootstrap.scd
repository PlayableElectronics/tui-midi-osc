// Index v1 sequencing-only bridge. Main.elapsedTime is the timestamp domain
// reported in /event/midi: monotonic seconds since the sclang process started.
var rustPort = "INDEX_RUST_PORT".getenv.asInteger;
var scPort = "INDEX_SC_PORT".getenv.asInteger;
var rust = NetAddr("127.0.0.1", rustPort);
var clock = TempoClock(2.0);
var staged, live, stream, runner, playing = false, active = Set.new;
var parseInts, parseFloats, parsePattern, emit, stop, activate, play, handle;

staged = (name: "bass", degrees: [0], durations: [1.0], velocities: [100], gate: 0.8, channel: 1, destination: "Monitor", revision: 0);
live = staged.copy;

parseInts = { |text| text.split($,).collect(_.asInteger) };
parseFloats = { |text| text.split($,).collect(_.asFloat) };
parsePattern = { |msg, offset, revision|
    var p = (name: msg[offset].asString, degrees: parseInts.(msg[offset + 1].asString), durations: parseFloats.(msg[offset + 2].asString), velocities: parseInts.(msg[offset + 3].asString), gate: msg[offset + 4].asFloat, channel: msg[offset + 5].asInteger, destination: msg[offset + 6].asString, revision: revision);
    if(p.degrees.isEmpty or: { p.degrees.size != p.durations.size or: { p.degrees.size != p.velocities.size } }) { Error("invalid pattern lengths").throw };
    if(p.durations.any({ |x| x <= 0 })) { Error("duration must be positive").throw };
    if(p.gate < 0 or: { p.gate > 1 } or: { p.channel < 1 or: { p.channel > 16 } }) { Error("invalid gate or channel").throw };
    p;
};
emit = { |kind, note, velocity, channel, destination|
    if(kind == "on") { active.add([note, channel, destination]) } { active.remove([note, channel, destination]) };
    rust.sendMsg("/index/v1/event/midi", Main.elapsedTime, kind, note, velocity, channel, destination);
};
stop = {
    playing = false;
    if(runner.notNil) { runner.stop; runner = nil };
    active.copy.do({ |x| emit.("off", x[0], 0, x[1], x[2]) });
    rust.sendMsg("/index/v1/state", 0);
};
activate = { |mode, req, revision|
    var doIt = { live = staged.copy; live.revision = revision; if(playing) { stop.(); play.() }; rust.sendMsg("/index/v1/pattern/committed", req, revision, mode, clock.beats) };
    if(mode == "bar") { clock.schedAbs(clock.nextBar, { doIt.(); nil }) } { doIt.() };
};
play = {
    if(playing.not) {
        playing = true;
        // Pbind is the source of coherent Events. It is never played through
        // Event.play, so no default synth or scsynth is involved.
        stream = Pbind(\degree, Pseq(live.degrees, inf), \dur, Pseq(live.durations, inf), \velocity, Pseq(live.velocities, inf), \gate, live.gate, \channel, live.channel, \destination, live.destination).asStream;
        runner = Routine({ var ev, note, beatDur;
            while { playing } {
                ev = stream.next(()); beatDur = ev.dur; note = (60 + ev.degree).clip(0, 127);
                emit.("on", note, ev.velocity, ev.channel, ev.destination);
                { (beatDur * ev.gate).wait; emit.("off", note, 0, ev.channel, ev.destination) }.fork(clock);
                beatDur.wait;
            }
        }).play(clock);
        rust.sendMsg("/index/v1/state", 1);
    }
};

handle = { |msg|
    var path = msg[0].asString;
    try {
        switch(path,
            "/index/v1/hello", { rust.sendMsg("/index/v1/ready", 1, Main.elapsedTime) },
            "/index/v1/project/sync", { var req = msg[1].asString; var tempo = msg[2].asFloat; staged = parsePattern.(msg, 3, 1); live = staged.copy; clock.tempo = tempo / 60; rust.sendMsg("/index/v1/project/sync_ack", req, 1, live.name) },
            "/index/v1/pattern/set", { var req = msg[1].asString; var rev = msg[2].asInteger; staged = parsePattern.(msg, 3, rev); rust.sendMsg("/index/v1/pattern/staged", req, rev) },
            "/index/v1/pattern/commit", { activate.(msg[3].asString, msg[1].asString, msg[2].asInteger) },
            "/index/v1/transport/play", { play.() },
            "/index/v1/transport/stop", { stop.() },
            "/index/v1/code/eval", { msg[1].asString.interpret },
            { Error("unknown OSC path: " ++ path).throw }
        );
    } { |err| rust.sendMsg("/index/v1/error", err.errorString) };
};
["/index/v1/hello", "/index/v1/project/sync", "/index/v1/pattern/set", "/index/v1/pattern/commit", "/index/v1/transport/play", "/index/v1/transport/stop", "/index/v1/code/eval"].do({ |path| OSCFunc(handle, path, recvPort: scPort) });
fork { loop { 1.wait } };
