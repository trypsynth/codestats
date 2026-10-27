% expect: lang="Prolog" total=12 code=7 comment=3 blank=2 shebang=0
/* List utilities. */
my_length([], 0).
my_length([_|T], N) :-
    my_length(T, N0),
    N is N0 + 1.

% Collect all members greater than a limit.
above(Limit, List, Result) :-
    findall(X, (member(X, List), X > Limit), Result).

?- above(2, [1, 2, 3, 4], R).
