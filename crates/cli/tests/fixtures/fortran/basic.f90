! expect: total=10 code=5 comment=3 blank=2 shebang=0
! simple fortran fixture

program hello
  character(len=5) :: msg
  msg = "hi"

! emit
  print *, msg
end program hello
