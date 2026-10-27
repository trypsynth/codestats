" expect: lang=Smalltalk total=11 code=9 comment=1 blank=1 shebang=0 "
Object subclass: #Account
    instanceVariableNames: 'balance'
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Banking'.

Account >> deposit: amount
    amount > 0 ifFalse: [^self error: 'Invalid amount'].
    balance := (balance ifNil: [0]) + amount.
    ^self
