# Make: a rule, a conditional and a define fold, as nvim-treesitter folds them
CC ?= cc
CFLAGS := -O2 -Wall  # f: none
export PATH := $(PATH):/opt/bin  # f: none

.PHONY: all clean  # f: none

all: build test  # f: 8-11
	@echo all  # f: 8-11

	@echo "done: ok"  # f: 8-11
# a comment after the recipe stays out  # f: none

build: main.o \
       util.o  # f: 14-17
	$(CC) -o app $^ \
ifeq looks like a conditional  # f: 14-17

%.o: %.c ; $(CC) -c $<  # f: 19-24
# a comment inside the recipe  # f: 19-24
ifeq ($(DEBUG),1)  # f: 21-24
	@echo debug build  # f: 19-24
	# a shell comment  # f: 19-24
endif  # f: 19-24

$(OBJS): %.o: %.c  # f: 26-27
	$(CC) -c $< -o $@  # f: 26-27
debug: CFLAGS += -g  # f: none

check: all  # f: 30-35
ifneq ($(CI),)  # f: 31-35
	./run-tests --ci  # f: 30-35
else  # f: 30-35
	./run-tests  # f: 30-35
endif  # f: 30-35
V ?= 0  # f: none
ifeq ($(OS),Windows_NT)  # f: 37-51
EXE := .exe  # f: none
else ifeq ($(shell uname),Darwin)  # f: none
EXE :=
  ifdef ARM  # f: 41-43
  ARCH := arm64  # f: none
  endif  # f: none
else  # f: none
	ifeq (tab,indented)  # f: none
	endif  # f: none
install:: all  # f: 47-49
	install -d $(PREFIX)/bin  # f: 47-49
	install app $(PREFIX)/bin  # f: 47-49
clean:  # f: none
endif  # f: none

define HELP  # f: 53-57
Usage: make [all|clean]  # f: 53-57
ifeq this is text  # f: 53-57
all: not a rule  # f: 53-57
endef  # f: 53-57

override define RUN  # f: 59-61
	$(CC) $(1)  # f: 59-61
endef

SRCS = a.c \
  b.c: c.c  # f: none

dist: all  # f: 66-67
	tar czf app.tgz app  # f: 66-67

