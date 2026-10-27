// expect: lang="Faust" total=9 code=5 comment=2 blank=2 shebang=0
// Simple sine synthesizer with a gain control.
declare name "synth";
import("stdfaust.lib");

freq = hslider("freq", 440, 50, 2000, 0.01);
gain = hslider("gain", 0.5, 0, 1, 0.01);

process = os.osc(freq) * gain <: _, _;
