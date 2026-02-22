# Set the location of the top of the MicroPython repository.
MICROPYTHON_TOP = ./micropython

# Include the main makefile fragment to build the MicroPython component.
include $(MICROPYTHON_TOP)/ports/embed/embed.mk
