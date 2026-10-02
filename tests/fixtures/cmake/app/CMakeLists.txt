# The app, under add_subdirectory(app).
add_executable(shop_app main.cpp)
target_link_libraries(shop_app PRIVATE Shop::core)
#                     ^ d: app/CMakeLists.txt:2
#                                      ^ d: CMakeLists.txt:27
#                                            ^ d: CMakeLists.txt:27
set(COUPON_CODE SAVE20)
