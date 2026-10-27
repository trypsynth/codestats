# expect: lang="E" total=13 code=10 comment=2 blank=1 shebang=0
# Simple E program with an object and a promise.
def makeCounter() {
    var count := 0
    return def counter {
        to increment() { count += 1 }
        to get() { return count }
    }
}

def c := makeCounter()
c.increment()
println(`count is ${c.get()}`)
