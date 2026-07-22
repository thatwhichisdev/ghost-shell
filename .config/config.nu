$env.config.show_banner = false
$env.config.buffer_editor = "hx"

$env.PROMPT_INDICATOR = ""
$env.PROMPT_INDICATOR_VI_INSERT = ": "
$env.PROMPT_INDICATOR_VI_NORMAL = "〉"
$env.PROMPT_MULTILINE_INDICATOR = "::: "

$env.STARSHIP_SHELL = "nu"

def create_left_prompt [] {
    starship prompt --cmd-duration $env.CMD_DURATION_MS $'--status=($env.LAST_EXIT_CODE)'
}

def create_right_prompt [] {
    starship prompt --right
}

$env.PROMPT_COMMAND = { || create_left_prompt }
$env.PROMPT_COMMAND_RIGHT = { || create_right_prompt }

def show_greeter [] {
    clear --keep-scrollback

    print $"|>    (ansi yellow_bold)Tools:(ansi reset)"
    print $"|>    (rustc --version)"
    print $"|>    (cargo --version)"
    print '|>'
}

show_greeter
