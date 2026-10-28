// expect: lang="OpenCL" total=8 code=5 comment=2 blank=1 shebang=0
__kernel void add(__global const float *a, __global float *out) {
	// Each work item handles one element.
	int i = get_global_id(0);
	out[i] = a[i] + 1.0f;
}

__kernel void noop(void) {}
