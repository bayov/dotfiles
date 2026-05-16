local bayov_color_scheme = require("bayov_color_scheme")
local helpers = require("helpers")

local module = {}

helpers.TintWhileKeyTableActive("search_mode", bayov_color_scheme.search_mode_bg)

return module
