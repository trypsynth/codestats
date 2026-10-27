# expect: lang="Octave" total=11 code=6 comment=3 blank=2 shebang=0
## Load a package and compute statistics.
pkg load statistics

function result = summarize(x)
  # Return the mean and standard deviation.
  result = [mean(x), std(x)];
endfunction

data = randn(100, 1);
printf("%f %f\n", summarize(data));
