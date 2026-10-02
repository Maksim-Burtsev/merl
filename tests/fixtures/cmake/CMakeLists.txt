# The shop of #307 in CMake (#432): every d case carries its answer under it.
cmake_minimum_required(VERSION 3.24)
project(shop CXX)
list(APPEND CMAKE_MODULE_PATH ${CMAKE_SOURCE_DIR}/cmake)
include(ShopHelpers)
#       ^ d: cmake/ShopHelpers.cmake:1
include(cmake/warnings.cmake)
#                ^ d: cmake/warnings.cmake:1
find_package(Tariff REQUIRED)
#            ^ d: cmake/FindTariff.cmake:1
option(SHOP_TESTS "Build the tests" ON)
set(SHOP_SOURCES src/order.cpp)
set(COUPON_CODE SAVE10)
shop_option(SHOP_DOCS OFF)
#  ^ d: cmake/ShopHelpers.cmake:8
#            ^ d: none

shop_add_library(shop_core ${SHOP_SOURCES})
# ^ d: cmake/ShopHelpers.cmake:2
# status: shop_add_library: by name, 1 match
#                 ^ d: none
#                              ^ d: CMakeLists.txt:12
SHOP_ADD_LIBRARY(shop_extra src/extra.cpp)
# ^ d: cmake/ShopHelpers.cmake:2
weigh_parcel(250)
# ^ d: cmake/ShopHelpers.cmake:12
add_library(Shop::core ALIAS shop_core)
#                      ^ d: none
add_library(shop-coupon STATIC src/coupon.cpp)
add_executable(shop.cli src/cli.cpp)
add_custom_target(docs COMMAND doxygen)
target_compile_options(shop_core PRIVATE ${SHOP_WARNINGS})
#                       ^ d: none
#                                 ^ d: none
#                                          ^ d: cmake/ShopHelpers.cmake:16
target_link_libraries(shop-coupon PRIVATE Shop::core Tariff::tariff)
#                          ^ d: CMakeLists.txt:29
#                                         ^ d: CMakeLists.txt:27
#                                               ^ d: CMakeLists.txt:27
#                                                            ^ d: cmake/FindTariff.cmake:2
add_dependencies(shop.cli docs)
#                    ^ d: CMakeLists.txt:30
#                         ^ d: CMakeLists.txt:31
add_custom_command(TARGET shop.cli POST_BUILD COMMAND $<TARGET_FILE:shop.cli>)
#                                                     ^ d: none
#                                                                     ^ d: CMakeLists.txt:30
set_target_properties(shop.cli PROPERTIES OUTPUT_NAME shop)
set_property(TARGET shop.cli PROPERTY CXX_STANDARD 20)
describe_coupon()
# ^ d: cmake/warnings.cmake:3
message(STATUS "rate ${TARIFF_RATE}, version ${SHOP_VERSION}")
#                       ^ d: cmake/warnings.cmake:2
#                                              ^ d: cmake/ShopHelpers.cmake:17
describe_tariff()
# ^ d: none
add_library(shop-gift INTERFACE)
#             ^ d: CMakeLists.txt:56
list(APPEND SHOP_SOURCES src/basket.cpp)
#             ^ d: CMakeLists.txt:12
if(SHOP_TESTS)
#     ^ d: CMakeLists.txt:11
  add_subdirectory(app)
#                  ^ d: app/CMakeLists.txt:1
endif()
set(ENV{SHOP_HOME} /opt/shop)
message(STATUS "$ENV{SHOP_HOME}")
#                     ^ d: none
unset(COUPON_CODE)
#       ^ d: picker CMakeLists.txt:13, app/CMakeLists.txt:7
