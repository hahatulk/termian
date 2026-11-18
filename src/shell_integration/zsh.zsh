# Termian shell integration for zsh: the legacy bash script's OSC 4545 events.
if [[ -z "$_termius_integration_installed" ]]; then
	_termius_integration_installed="yes"
	builtin printf "\e]4545;A\a"

	_termius_encode() {
		builtin printf '%s' "$1" | command base64 | command tr -d '\n'
	}

	_termius_current_command=""
	_termius_original_PS1=""
	_termius_custom_PS1=""

	_termius_wrap_prompt() {
		# Re-wrap when the user (or a theme) set a new PS1.
		if [[ "$PS1" != "$_termius_custom_PS1" ]]; then
			_termius_original_PS1="$PS1"
			_termius_custom_PS1="%{$(builtin printf '\e]4545;ShellPromptBegins\a')%}${_termius_original_PS1}%{$(builtin printf '\e]4545;ShellPromptEnds\a')%}"
			PS1="$_termius_custom_PS1"
		fi
	}

	_termius_preexec() {
		_termius_current_command="$1"
		builtin printf "\e]4545;CommandStarted;%s\a" "$(_termius_encode "$1")"
	}

	_termius_precmd() {
		local termius_status=$?
		if [[ -n "$_termius_current_command" ]]; then
			builtin printf "\e]4545;CommandExited;%s\a" "$termius_status"
		fi
		_termius_current_command=""
		builtin printf "\e]4545;SetCwd;%s\a" "$(_termius_encode "$PWD")"
		_termius_wrap_prompt
	}

	autoload -Uz add-zsh-hook
	add-zsh-hook preexec _termius_preexec
	add-zsh-hook precmd _termius_precmd
	builtin printf "\e]4545;B\a"
fi
