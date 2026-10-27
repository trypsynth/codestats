// expect: lang="SuperCollider" total=11 code=7 comment=2 blank=2 shebang=0
// Define and play a simple synth.
s.boot;

SynthDef(\beep, { |out = 0, freq = 440, amp = 0.2|
	var sig = SinOsc.ar(freq) * EnvGen.kr(Env.perc, doneAction: 2);
	Out.ar(out, sig * amp ! 2);
}).add;

~notes = [60, 64, 67];
~notes.do { |n| Synth(\beep, [\freq, n.midicps]) };
