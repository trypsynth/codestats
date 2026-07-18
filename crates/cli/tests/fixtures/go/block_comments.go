// expect: total=12 code=5 comment=4 blank=3 shebang=0
// go block comment fixture

package main

/* block comment
   continues here */
import "fmt"

func main() {
	fmt.Println("hi")
}
