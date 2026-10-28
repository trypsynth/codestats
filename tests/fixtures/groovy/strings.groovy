// expect: lang="Groovy" total=5 code=4 comment=1 blank=0 shebang=0
def glob = 'src/**/*.groovy'
def text = '''
/* still a string '''
println glob
