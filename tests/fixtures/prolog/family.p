% expect: lang="Prolog" total=14 code=10 comment=2 blank=2 shebang=0
% Family relationships.
parent(tom, bob).
parent(bob, ann).
parent(bob, pat).

grandparent(X, Z) :-
    parent(X, Y),
    parent(Y, Z).

:- initialization(main).
main :-
    findall(G, grandparent(tom, G), Gs),
    print(Gs), nl.
