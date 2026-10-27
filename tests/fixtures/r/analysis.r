# expect: lang="R" total=13 code=8 comment=2 blank=3 shebang=0
# Fit a linear model and plot the result.
library(ggplot2)

df <- data.frame(x = 1:10, y = (1:10) * 2 + rnorm(10))
model <- lm(y ~ x, data = df)

summarize <- function(m) {
  coef(m)
}

print(summarize(model))
ggplot(df, aes(x, y)) + geom_point()
