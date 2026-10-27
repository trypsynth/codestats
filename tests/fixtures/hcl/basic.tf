# expect: total=9 code=4 comment=3 blank=2 shebang=0
# simple terraform fixture

variable "name" {
	type = string
}

# output
output "greeting" { value = "hi ${var.name}" }
