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

    print '|>  _____ _   _ _____ _____ _____   _____ _   _ _____ _     _     '
    print '|> |  __ | | | |  _  /  ___|_   _| /  ___| | | |  ___| |   | |    '
    print '|> | |  \| |_| | | | \ `--.  | |   \ `--.| |_| | |__ | |   | |    '
    print '|> | | __|  _  | | | |`--. \ | |    `--. |  _  |  __|| |   | |    '
    print '|> | |_\ | | | \ \_/ /\__/ / | |   /\__/ | | | | |___| |___| |____'
    print '|>  \____\_| |_/\___/\____/  \_/   \____/\_| |_\____/\_____\_____/'
    print '|>                                                                '

    print $"|>    (ansi yellow_bold)Tools:(ansi reset)"
    print $"|>    (rustc --version)"
    print $"|>    (cargo --version)"
    print '|>'
}

show_greeter
