// expect: lang="Swig" total=17 code=10 comment=3 blank=4 shebang=0
/* SWIG interface for the example library. */
%module example

%{
#include "example.h"
%}

// Map std::string to Python str.
%include "std_string.i"
%apply int *OUTPUT { int *result };

%typemap(in) double {
  $1 = PyFloat_AsDouble($input);
}

int add(int a, int b);
