# Helpers the shop's CMakeLists.txt includes (#432).
function(shop_add_library name)
  add_library(${name} STATIC ${ARGN})
#               ^ d: none
  target_compile_features(${name} PUBLIC cxx_std_20)
endfunction()

macro(shop_option name default)
  option(${name} "Shop option ${name}" ${default})
endmacro()

FUNCTION(Weigh_Parcel grams)
  set(PARCEL_GRAMS ${grams} PARENT_SCOPE)
ENDFUNCTION()

set(SHOP_WARNINGS -Wall -Wextra)
set(SHOP_VERSION 1.2 CACHE STRING "The shop's version")

#[[ A bracket comment that reads like declarations:
function(describe_tariff)
set(TARIFF_RATE 3)
]]
#[==[ and one with a level, a `]]` inside:
add_library(shop-gift STATIC gift.cpp) ]]
]==]
set(COURIER_NOTE [=[
macro(describe_tariff)
]=])
message(STATUS "a quoted argument over lines:
set(TARIFF_RATE 4)
")
