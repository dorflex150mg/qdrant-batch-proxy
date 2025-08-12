#install.packages("ggplot2")
#install.packages("tidyr")
#install.packages("dplyr")

library(ggplot2)
library(tidyr)
library(dplyr)

bulk = read.csv("partials_bulk.csv") 
single = read.csv("partials_single.csv")
df = data.frame(PartialSingle=single[,1], PartialBulk=bulk[,1])
df_long <- df %>%
    pivot_longer(
                 cols = c(PartialSingle, PartialBulk),
                 names_to = "Category",
                 values_to = "Elapsed"
    ) %>% 
    mutate(Elapsed= as.numeric(Elapsed)/1000)
df_long
png("partials_density_seconds.png")
ggplot(df_long, aes(x = Elapsed, fill = Category)) + geom_density(alpha=0.8) + scale_x_log10() + labs(x = "Elapsed (thousands)", y = "Density")
dev.off()
