function gemini --wraps='firejail --whitelist=~/.gemini --whitelist=$PWD gemini' --description 'sandboxed gemini-cli'
  firejail --whitelist=~/.gemini --whitelist=$PWD gemini $argv
end
