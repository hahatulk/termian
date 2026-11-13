# Termian shell integration for fish: the legacy bash script's OSC 4545 events.
if not set -q _termius_integration_installed
    set -g _termius_integration_installed yes
    printf '\e]4545;A\a'

    function _termius_encode
        printf '%s' $argv[1] | command base64 | string join ''
    end

    function _termius_preexec --on-event fish_preexec
        set -g _termius_current_command $argv[1]
        printf '\e]4545;CommandStarted;%s\a' (_termius_encode $argv[1])
    end

    function _termius_postexec --on-event fish_postexec
        printf '\e]4545;CommandExited;%s\a' $status
        set -e _termius_current_command
    end

    function _termius_cwd --on-event fish_prompt
        printf '\e]4545;SetCwd;%s\a' (_termius_encode $PWD)
    end

    functions -c fish_prompt _termius_original_fish_prompt
    function fish_prompt
        printf '\e]4545;ShellPromptBegins\a'
        _termius_original_fish_prompt
        printf '\e]4545;ShellPromptEnds\a'
    end
    printf '\e]4545;B\a'
end
