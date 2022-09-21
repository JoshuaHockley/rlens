-- Utils

-- Execute a command line in the shell
function exec(cmd)
    os.execute(cmd)
end

-- Execute a command line in the shell and return the output from stdout
-- (May not work on some platforms)
function exec_read(cmd)
    local handle = io.popen(cmd, 'r')
    local out = handle:read('*a')
    handle:close()
    return out
end

-- Quote a string to be interpreted by the shell without any expansions
-- This should be sufficient for passing filepaths into the shell via exec/exec_read
function shquote(s)
    return [[']] .. s:gsub([[']], [['\'']]) .. [[']]
end


-- Colors

function grey(component, alpha)
    return {
        r = component,
        g = component,
        b = component,
        a = alpha,
    }
end

gruvbox_component = 0.15686

rlens.bg_color(grey(gruvbox_component, 0.9))
rlens.gallery_cursor_color(grey(0.4, 0.8))
rlens.gallery_border_color(grey(0.7, 0.9))
rlens.status_bar_color(grey(0.05, 0.7))


-- General

rlens.preload_range(5, 5)

-- Uncomment this to save generated thumbnails to the thumbnail directory
--rlens.save_thumbnails(true)

rlens.gallery_tile_width(200)
rlens.gallery_height_width_ratio(1)

rlens.status_bar_position('bottom')


-- Scaling / align

scaling_modes = {
    -- Fit the whole image to the window
    function()
        rlens.scaling('fit')
        rlens.align_x('center')
        rlens.align_y('center')
    end,

    -- Fit to the width of the image and center at its top
    function()
        rlens.scaling('fit_width')
        rlens.align_x('center')
        rlens.align_y('top')
    end,
}

scaling_mode = 1

function set_scaling()
    rlens.freeze()
    scaling_modes[scaling_mode]()
    rlens.unfreeze()
end
set_scaling()

function next_scaling_mode()
    scaling = scaling % #scaling_modes + 1
end


-- Status bar

function query.status_bar()
    local image = rlens.current_image()

    local left = '[' .. rlens.index() .. '/' .. rlens.total_images() .. '] ' .. image.filestem

    local right = ''
    local metadata = image.metadata
    if (metadata ~= nil) then
        local dimensions = metadata.dimensions
        right = dimensions.width .. 'x' .. dimensions.height .. ' [' .. metadata.format .. ']'
    end

    return left, right
end


-- Keybinds

-- General
bind('q', rlens.exit)
bind('f', rlens.toggle_fullscreen)
bind('d', rlens.toggle_status_bar)
bind('S-r', rlens.reload)

-- Mode changing
bind_image('Tab', function() rlens.mode('gallery') end)
bind_gallery('Tab', function() rlens.mode('image') end)
bind_gallery('Return', rlens.select)

-- Image mode navigation
bind_image('Left', rlens.prev_wrapping)
bind_image('Right', rlens.next_wrapping)
bind_image('p', rlens.prev_wrapping)
bind_image('n', rlens.next_wrapping)
bind_image('g', rlens.first)
bind_image('S-g', rlens.last)

-- Gallery mode navigation
bind_gallery('Left', rlens.gallery_prev)
bind_gallery('Right', rlens.gallery_next)
bind_gallery('Up', rlens.gallery_up)
bind_gallery('Down', rlens.gallery_down)
bind_gallery('h', rlens.gallery_prev)
bind_gallery('l', rlens.gallery_next)
bind_gallery('k', rlens.gallery_up)
bind_gallery('j', rlens.gallery_down)
bind_gallery('g', rlens.gallery_first)
bind_gallery('S-g', rlens.gallery_last)

-- Image transforms
pan_amt = 20
bind_image('h', function() rlens.pan(pan_amt, 0) end)
bind_image('j', function() rlens.pan(0, -pan_amt) end)
bind_image('k', function() rlens.pan(0, pan_amt) end)
bind_image('l', function() rlens.pan(-pan_amt, 0) end)
bind_image('S-h', function() rlens.pan(pan_amt/3, 0) end)
bind_image('S-j', function() rlens.pan(0, -pan_amt/3) end)
bind_image('S-k', function() rlens.pan(0, pan_amt/3) end)
bind_image('S-l', function() rlens.pan(-pan_amt/3, 0) end)

zoom_amt = 1.05
bind_image('i', function() rlens.zoom(zoom_amt) end)
bind_image('o', function() rlens.zoom(-zoom_amt) end)
bind_image('=', function() rlens.zoom(zoom_amt) end)
bind_image('-', function() rlens.zoom(-zoom_amt) end)

rotate_amt = 5
bind_image(',', function() rlens.rotate(-rotate_amt) end)
bind_image('.', function() rlens.rotate(rotate_amt) end)

bind_image('b', rlens.hflip)
bind_image('v', rlens.vflip)

bind_image('r', rlens.reset)

bind_image('s', next_scaling_mode)


-- Hooks

function hook.current_image_change()
    rlens.refresh_status_bar()
end

function hook.transform_update()
end

function hook.current_image_load()
    rlens.refresh_status_bar()
end

function hook.resize()
    rlens.reset()
end

