% expect: lang="MATLAB" total=12 code=10 comment=2 blank=0 shebang=0
% Plot a damped sine wave.
function plot_wave(n)
    t = linspace(0, 10, n);
    y = exp(-t / 3) .* sin(2 * pi * t);
    if n > 100
        disp('Plotting many points');
    elseif n > 10
        disp('Plotting some points');
    end
    plot(t, y);
end
