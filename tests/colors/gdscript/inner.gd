extends Node
# A plain comment, with TODO: notes


class Inner extends RefCounted:
	var value = 0x1F + 0b101 + 1_000 + 1e3

	func _init(v = 0):
		value = v
		super._init()

	func describe():
		return """A
multiline string""" + r"raw\n"


func _process(_delta):
	var inner = Inner.new(3)
	var arr = [1, 2.5, -3, inner, PI, TAU, INF, NAN]
	if arr.size() > 2 && !arr.is_empty() || false:
		breakpoint
	get_node("../Other").queue_free()
	preload("res://scene.tscn")
