' expect: lang="Visual Basic/Visual Basic .NET" total=18 code=13 comment=2 blank=3 shebang=0
VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "Account"
Option Explicit

' Current balance of the account.
Private mBalance As Currency

Public Sub Deposit(ByVal Amount As Currency)
    If Amount > 0 Then mBalance = mBalance + Amount
End Sub

Public Function Balance() As Currency
    Balance = mBalance
End Function
