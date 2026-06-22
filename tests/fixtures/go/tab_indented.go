// expect: total=12 code=6 comment=3 blank=3 shebang=0
// tab-indented go fixture

package main

import "fmt"

// Greet returns a greeting.
func greet(name string) string {
	return fmt.Sprintf("Hello, %s!", name)
}
func main() { fmt.Println(greet("world")) }
