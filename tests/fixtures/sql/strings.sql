-- expect: lang="SQL" total=5 code=3 comment=2 blank=0 shebang=0
SELECT 'it''s -- not a comment' AS note,
       '/* nor this */' AS other
FROM t;
/* a real comment */
