// expect: total=11 code=6 comment=3 blank=2 shebang=0
// simple glsl fragment shader

precision mediump float;
uniform vec3 color;

/* main entry point */
void main() {
	gl_FragColor = vec4(color, 1.0);
	gl_FragColor.r *= 0.5;
}
