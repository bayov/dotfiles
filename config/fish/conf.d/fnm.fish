# fnm
set FNM_PATH "/home/bayov/.local/share/fnm"
if [ -d "$FNM_PATH" ]
  fish_add_path "$FNM_PATH"
  fnm env | source
end
