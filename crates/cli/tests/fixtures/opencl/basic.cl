/* expect: total=9 code=5 comment=2 blank=2 shebang=0
 * simple opencl fixture */

__kernel void add(__global float* data) {
	int id = get_global_id(0);
	data[id] = data[id] + 1.0f;
}

barrier(CLK_GLOBAL_MEM_FENCE);
