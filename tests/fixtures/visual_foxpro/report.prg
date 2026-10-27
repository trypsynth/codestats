* expect: lang="Visual FoxPro" total=17 code=12 comment=2 blank=3 shebang=0
* Summarize open orders by customer.
LOCAL lnTotal
lnTotal = 0

USE orders
SCAN FOR status = "OPEN"
	lnTotal = lnTotal + amount
ENDSCAN

DO WHILE lnTotal > 1000
	lnTotal = lnTotal - 1000
ENDDO

DEFINE CLASS Summary AS Custom
	Total = 0
ENDDEFINE
