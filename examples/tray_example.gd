extends Node

var tray_icon: TrayIcon
var window_visible: bool = true

func _ready():
	tray_icon = TrayIcon.new()
	add_child(tray_icon)

	tray_icon.set_tray_id("my_application_tray")
	tray_icon.set_title("My Application")

	# Method 1: Load from resource path
	if tray_icon.set_icon_from_path("res://icon.svg"):
		print("Icon loaded successfully")
	else:
		tray_icon.set_icon_name("application-x-executable")
		print("Using fallback icon")

	# Method 2: Load from Texture2D
	# var texture = load("res://icon.svg")
	# if texture:
	#     tray_icon.set_icon_from_texture(texture)

	# Method 3: Load from Image
	# var texture = load("res://icon.svg")
	# if texture:
	#     var image = texture.get_image()
	#     tray_icon.set_icon_from_image(image)

	tray_icon.set_tooltip("My Application", "Running in background", "")

	build_menu()

	tray_icon.menu_activated.connect(_on_menu_activated)
	tray_icon.checkmark_toggled.connect(_on_checkmark_toggled)
	tray_icon.radio_selected.connect(_on_radio_selected)

	if tray_icon.spawn_tray():
		print("Tray icon created successfully")
	else:
		print("Failed to create tray icon")

func build_menu():
	tray_icon.clear_menu()

	tray_icon.add_menu_item("show_window", "Show Window", "", true, true)
	tray_icon.add_menu_item("hide_window", "Hide Window", "", true, true)

	tray_icon.add_separator()

	tray_icon.begin_submenu("Settings", "preferences-system", true, true)
	tray_icon.add_submenu_checkmark("Settings", "always_on_top", "Always on Top", "", false, true, true)
	tray_icon.add_submenu_checkmark("Settings", "borderless", "Borderless", "", false, true, true)
	tray_icon.add_submenu_separator("Settings")
	tray_icon.add_submenu_item("Settings", "preferences", "Preferences", "", true, true)

	tray_icon.add_separator()

	tray_icon.add_radio_group("theme", 0)
	tray_icon.add_radio_option("theme", "light", "Light Theme", "", true, true)
	tray_icon.add_radio_option("theme", "dark", "Dark Theme", "", true, true)
	tray_icon.add_radio_option("theme", "auto", "Auto Theme", "", true, true)

	tray_icon.add_separator()

	tray_icon.add_checkmark_item("sound_enabled", "Enable Sound", "audio-volume-high", true, true, true)

	tray_icon.add_separator()

	tray_icon.begin_submenu("Help", "help-browser", true, true)
	tray_icon.add_submenu_item("Help", "about", "About", "help-about", true, true)
	tray_icon.add_submenu_item("Help", "documentation", "Documentation", "help-contents", true, true)

	tray_icon.add_separator()

	tray_icon.add_menu_item("quit", "Quit", "application-exit", true, true)

func _on_menu_activated(id: String):
	print("Menu activated: ", id)

	match id:
		"show_window":
			show_window()
		"hide_window":
			hide_window()
		"preferences":
			print("Open preferences")
		"about":
			print("Show about dialog")
		"documentation":
			OS.shell_open("https://github.com/yuna0x0/godot-ksni")
		"quit":
			get_tree().quit()

func _on_checkmark_toggled(id: String, checked: bool):
	print("Checkmark toggled: ", id, " = ", checked)

	match id:
		"always_on_top":
			get_window().always_on_top = checked
		"borderless":
			get_window().borderless = checked
		"sound_enabled":
			AudioServer.set_bus_mute(AudioServer.get_bus_index("Master"), not checked)

func _on_radio_selected(group_id: String, index: int, option_id: String):
	print("Radio selected: ", group_id, "[", index, "] = ", option_id)

	match group_id:
		"theme":
			match option_id:
				"light":
					print("Apply light theme")
				"dark":
					print("Apply dark theme")
				"auto":
					print("Apply auto theme")

func show_window():
	if not window_visible:
		get_window().visible = true
		window_visible = true

func hide_window():
	if window_visible:
		get_window().visible = false
		window_visible = false
