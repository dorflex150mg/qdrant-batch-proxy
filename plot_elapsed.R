library(ggplot2)

latency_single = read.csv("finals_single.csv")
elapsed_single = latency_single[,1][1]
absolutes_bulk = read.csv("absolutes_bulk.csv")
elapsed_bulk = absolutes_bulk[,1][999] - absolutes_bulk[,1][1]
df = data.frame(Category = c("ElapsedSingle", "ElapsedBulk"), Time = c(elapsed_single/1000000, elapsed_bulk/100000))
png("elapsed_millis.png")
ggplot(df, aes(x = Category, y = Time)) + geom_col() + labs(y = "Elapsed Time 100 requests (Nanos)", x = "Category")
dev.off()


