! expect: lang="Fortran" total=9 code=7 comment=1 blank=1 shebang=0
module shapes
implicit none
contains

character(len=5) function label()
label = 'shape'
end function label
end module shapes
