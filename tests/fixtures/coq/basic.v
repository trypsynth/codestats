(* expect: lang="Coq" total=15 code=10 comment=2 blank=3 shebang=0 *)
(* Natural number addition is commutative. *)
Require Import Arith.

Inductive color : Type :=
  | red
  | green.

Definition double (n : nat) : nat := n + n.

Theorem plus_comm : forall n m : nat, n + m = m + n.
Proof.
  intros n m.
  apply Nat.add_comm.
Qed.
