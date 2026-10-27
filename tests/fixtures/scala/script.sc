// expect: lang="Scala" total=14 code=9 comment=2 blank=3 shebang=0
// Ammonite script that summarizes some numbers.
case class Stats(min: Int, max: Int)

object Summary {
  def of(xs: Seq[Int]): Stats = Stats(xs.min, xs.max)
}

trait Printer {
  def show(s: Stats): String = s"${s.min}..${s.max}"
}

val nums = List(3, 1, 4, 1, 5)
println(Summary.of(nums))
