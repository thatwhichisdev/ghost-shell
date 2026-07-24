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

    print '|>   ____ _   _  ___  ____ _____   ____  _   _ _____ _     _     '
    print '|>  / ___| | | |/ _ \/ ___|_   _| / ___|| | | | ____| |   | |    '
    print '|> | |  _| |_| | | | \___ \ | |   \___ \| |_| |  _| | |   | |    '
    print '|> | |_| |  _  | |_| |___) || |    ___) |  _  | |___| |___| |___ '
    print '|>  \____|_| |_|\___/|____/ |_|   |____/|_| |_|_____|_____|_____|'
    print '|>                                                               '

    print $"|>    (ansi yellow_bold)Tools:(ansi reset)"
    print $"|>    (rustc --version)"
    print $"|>    (cargo --version)"

    print '|>                                                               '
}

show_greeter
