# expect: lang="Nim" total=6 code=5 comment=1 blank=0 shebang=0
let tag = "#not a comment"
let doc = """
# also inside the string
"""
echo tag
