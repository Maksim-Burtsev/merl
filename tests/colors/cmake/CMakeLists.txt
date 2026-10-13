#!/usr/bin/env -S cmake -P
# Ledger: a small project to cover the CMake grammar.
cmake_minimum_required(VERSION 3.20)
project(Ledger VERSION 1.4.2 LANGUAGES C CXX)

#[[ A bracket comment
    over two lines ]]

option(LEDGER_TESTS "Build the tests" ON)
option(LEDGER_DOCS "Build the docs" off)
set(CMAKE_CXX_STANDARD 20)
set(LEDGER_PREFIX "${CMAKE_INSTALL_PREFIX}/share/ledger" CACHE PATH "Where data goes")
set(LEDGER_NAME [=[ledger "core"]=])
set(ENV{LEDGER_HOME} "$ENV{HOME}/.ledger")
unset(LEDGER_OLD CACHE)
unset(LEDGER_TMP PARENT_SCOPE)

list(APPEND LEDGER_SOURCES src/main.cpp src/book.cpp)
list(LENGTH LEDGER_SOURCES count)
list(TRANSFORM LEDGER_SOURCES PREPEND "${CMAKE_CURRENT_SOURCE_DIR}/")
list(TRANSFORM LEDGER_SOURCES REPLACE "\\.cpp$" ".cc" REGEX)
list(TRANSFORM LEDGER_SOURCES OUTPUT_VARIABLE copied)

if(NOT DEFINED LEDGER_VERSION AND LEDGER_TESTS)
  message(STATUS "Version ${PROJECT_VERSION} at ${LEDGER_PREFIX}")
elseif(CMAKE_SYSTEM_NAME STREQUAL "Darwin" OR EXISTS /opt/homebrew)
  message(WARNING "macOS: \"brew\" paths\tadded")
else()
  message(FATAL_ERROR "unsupported")
endif()

function(ledger_add_target name kind)
  if(kind MATCHES "^lib")
    add_library(${name} STATIC ${ARGN})
  else()
    add_executable(${name} ${ARGN})
  endif()
  target_include_directories(${name} PRIVATE include)
  return()
endfunction()

macro(ledger_warn text)
  message(AUTHOR_WARNING ${text})
endmacro()

foreach(file IN LISTS LEDGER_SOURCES)
  if(file MATCHES "test")
    continue()
  endif()
  get_filename_component(base ${file} NAME_WE)
endforeach()

set(i 0)
while(i LESS 3)
  math(EXPR i "${i} + 1")
  if(i EQUAL 2)
    break()
  endif()
endwhile()

block(SCOPE_FOR VARIABLES POLICIES PROPAGATE LEDGER_NAME)
  set(LEDGER_NAME "inner")
endblock()

ledger_add_target(ledger_core libcore ${LEDGER_SOURCES})
target_link_libraries(ledger_core PUBLIC Threads::Threads $<$<CONFIG:Debug>:asan>)
set_target_properties(ledger_core PROPERTIES OUTPUT_NAME ledger POSITION_INDEPENDENT_CODE TRUE)
install(TARGETS ledger_core DESTINATION lib)
if(LEDGER_LIB-NOTFOUND OR no)
  include(CTest)
endif()
