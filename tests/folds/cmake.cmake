# CMake: if, foreach, while, function, macro and block fold
cmake_minimum_required(VERSION 3.20)  # f: none
project(Demo LANGUAGES C)  # f: none

#[[ a bracket comment
if(NOT_CODE)
endif()
]]

#[==[
function(not_code)
]]
endfunction()
]==]

set(DOC [=[
if(also_not_code)
]=])  # f: none

set(MSG "endif() inside a string \"
foreach(x)")  # f: none

function(add_demo name)  # f: 23-40
  set(srcs ${ARGN})  # f: 23-40
  if(WIN32)  # f: 25-29
    list(APPEND srcs win.c)  # f: 23-40
  elseif(APPLE)  # f: 23-40
    list(APPEND srcs mac.c)  # f: 23-40
  endif()  # f: 23-40
  foreach(src IN LISTS srcs)  # f: 30-34
    if(src MATCHES "\\.c$")  # f: 31-33
      message(STATUS "C: ${src}")  # f: 23-40
    endif()  # f: 23-40
  endforeach()  # f: 23-40
  add_executable(${name}  # f: 23-40
    ${srcs}  # f: 23-40
  )  # f: 23-40
  # endfunction() in a comment  # f: 23-40
  target_link_libraries(${name} PRIVATE m)  # f: 23-40
endfunction()  # f: 23-40

macro(warn msg)  # f: 42-44
  message(WARNING "${msg}")  # f: 42-44
endmacro()  # f: 42-44

IF(UNIX)  # f: 46-52
  set(i 0)  # f: none
  WHILE(i LESS 3)  # f: 48-51
    math(EXPR i "${i} + 1")  # f: none
    add_demo(demo${i} main.c)  # f: none
  ENDWHILE()  # f: none
ENDIF()  # f: none

block (SCOPE_FOR VARIABLES)  # f: 54-57
  set(x 1)  # f: none
  if_ok(x)  # f: none
endblock()  # f: none

if(A AND  # f: 59-62
   B)  # f: none
  message(STATUS ab)  # f: none
endif(A AND B)  # f: none

foreach(x a b)  # f: 64-67
  message(${x})  # f: none
endforeach(
  x)  # f: none
