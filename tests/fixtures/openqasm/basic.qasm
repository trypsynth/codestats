// expect: total=10 code=5 comment=2 blank=3 shebang=0
// simple openqasm fixture

OPENQASM 2.0;
qreg q[2];

creg c[2];
h q[0];

measure q -> c;
