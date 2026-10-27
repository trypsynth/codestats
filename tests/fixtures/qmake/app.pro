# expect: lang="QMake" total=13 code=10 comment=2 blank=1 shebang=0
# Project file for the example application.
TEMPLATE = app
TARGET = example
QT += core gui widgets
CONFIG += c++17

INCLUDEPATH += include
SOURCES += \
    src/main.cpp \
    src/window.cpp
HEADERS += \
    include/window.h
