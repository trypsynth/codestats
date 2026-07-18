# expect: total=8 code=3 comment=3 blank=2 shebang=0
# simple cmake fixture

cmake_minimum_required(VERSION 3.22)
project(Demo)

# comment about target
add_executable(demo main.cpp)
