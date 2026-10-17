@tool
class_name Player
extends CharacterBody2D
## A player that walks and jumps.

signal health_changed(old_value: int, new_value: int)
signal died

enum State { IDLE, RUN, JUMP = 4 }

const SPEED := 300.0
const GRAVITY = 980
@export var max_health: int = 100
@export_range(0, 10, 0.5) var jump_power := 4.5
@onready var sprite: Sprite2D = $Sprite2D
@onready var label = %HealthLabel
var state: State = State.IDLE
var items: Array[String] = ["sword", 'shield']
var lookup := {"a": 1, b = 2}
static var count := 0

var health: int = 100:
	set(value):
		var old := health
		health = clampi(value, 0, max_health)
		health_changed.emit(old, health)
	get:
		return health


func _ready() -> void:
	sprite.modulate = Color(1, 0.5, 0.25)
	health_changed.connect(_on_health_changed)
	print("ready %s" % name)
	var node_path := ^"Path/To/Node"
	var string_name := &"jump"
	assert(self is CharacterBody2D)


func _physics_process(delta: float) -> void:
	if not is_on_floor():
		velocity.y += GRAVITY * delta
	elif Input.is_action_just_pressed("jump") and state != State.JUMP:
		velocity.y = -jump_power * 100
	var dir := Input.get_axis(&"left", &"right")
	velocity.x = dir * SPEED if dir else move_toward(velocity.x, 0, SPEED)
	move_and_slide()


func _on_health_changed(_old: int, new: int) -> void:
	match new:
		0:
			died.emit()
		var x when x < 10:
			pass
		_:
			label.text = str(new)


static func make(parent: Node) -> Player:
	var p := Player.new()
	parent.add_child(p)
	for i in range(3):
		while count < i:
			count += 1
			continue
		break
	return p


func take(amount = 1, ...rest) -> bool:
	var f := func(a): return a * 2
	await get_tree().create_timer(0.5).timeout
	health -= f.call(amount)
	return health <= 0 and null == null or true
