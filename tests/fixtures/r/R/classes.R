# S4 and reference classes: a generic with two methods.
setClass("Shape", representation("VIRTUAL"))
setClass("Circle", contains = "Shape", representation(r = "numeric"))
#                               ^ d: R/classes.R:2
setGeneric("area", function(shape) standardGeneric("area"))
setMethod("area", "Circle", function(shape) pi * shape@r^2)
setClass("Square", contains = "Shape", representation(side = "numeric"))
setMethod("area", "Square", function(shape) shape@side^2)
Account <- setRefClass("Account", fields = list(balance = "numeric"))
