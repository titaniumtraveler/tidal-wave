local function proto_fields(opts)
  local prefix = opts[2]
  local proto = Proto.new(prefix, opts[1])

  local bytes = ProtoField.bytes("usb.elgato.trailing_bytes", "bytes")
  proto.fields.bytes = bytes
  function proto.dissector(buf, pinfo, tree)
    local subtree = tree:add(proto)

    for _, subopts in pairs(opts.fields) do
      local name = subopts[2]
      local range = buf:range(subopts.range[1], subopts.range[2])

      if type(subopts.decode) == "function" then
        subtree:add(proto.fields[name], range, subopts.decode(range))
      elseif subopts.proto then
        subopts.proto.dissector(range:tvb(), pinfo, subtree)
      else
        subtree:add(proto.fields[name], range)
      end
    end
  end

  for _, subopts in pairs(opts.fields) do
    local name = subopts[2]
    local abbr = ("%s.%s"):format(prefix, name)
    proto.fields[name] = ProtoField.new( --
      subopts[1],
      abbr,
      subopts.type,
      subopts.valuestring,
      subopts.base,
      subopts.mask,
      subopts.desc
    )

    if subopts.fields then
      subopts[2] = abbr
      subopts.proto = proto_fields(subopts)
    end
  end

  return proto
end

-- For some reason the setup header isn't parsed by wireguard when hooking into
-- this, so we just parse it manually ourselves
local header = proto_fields {
  "USB Control Request Header",
  "usb.elgato.header",
  type = ftypes.PROTOCOL,
  range = { 0, 7 },

  fields = {
    {
      "bRequest",
      "bRequest",
      type = ftypes.UINT8,
      range = { 0, 1 },
    },
    {
      "wValue",
      "wValue",
      type = ftypes.UINT16,
      range = { 1, 2 },
      base = base.HEX,
      valuestring = {
        [0x0002] = "persistent",
      },
      decode = function(buf)
        return buf:le_uint()
      end,
    },
    {
      "wIndex",
      "wIndex",
      type = ftypes.UINT16,
      range = { 3, 2 },
    },
    {
      "wLength",
      "wLength",
      type = ftypes.UINT16,
      range = { 5, 2 },
      decode = function(bus)
        return bus:le_uint()
      end,
    },
  },
}

local config = proto_fields {
  "USB Elgato Wave XLR Config",
  "usb.elgato.config",
  type = ftypes.PROTOCOL,
  range = { 7, 34 },

  fields = {
    {
      "Input Gain",
      "gain",
      type = ftypes.FLOAT,
      range = { 0, 2 },
      desc = "Input gain in dB",
      base = base.NONE,
      decode = function(buf)
        return buf:le_uint() / 256
      end,
    },

    {
      "Unknown at bytes 2-3",
      "unknown-2",
      type = ftypes.NONE,
      range = { 2, 2 },
    },

    {
      "Mute",
      "mute",
      type = ftypes.BOOLEAN,
      range = { 4, 1 },
      base = 1,
      mask = 0x01,
    },
    {
      "Clipguard",
      "clipguard",
      type = ftypes.BOOLEAN,
      range = { 5, 1 },
      base = 1,
      mask = 0x01,
    },
    {
      "Phantom Power",
      "phantom",
      type = ftypes.BOOLEAN,
      range = { 6, 1 },
      desc = "48V Phantom Power",
      base = 1,
      mask = 0x01,
    },
    {
      "Lowcut Filter",
      "lowcut",
      type = ftypes.UINT16,
      range = { 7, 2 },
      valuestring = {
        [0x0000] = "Off",
        [0x0100] = "80Hz",
        [0x0001] = "120Hz",
      },
      base = base.HEX,
      decode = function(buf)
        buf:le_uint()
      end,
    },
    {
      "Monitor Volume",
      "volume",
      type = ftypes.FLOAT,
      range = { 9, 2 },
      desc = "Monitor volume in dB. Range 0dB to -128dB",
      decode = function(buf)
        return buf:le_int() / 256
      end,
    },

    {
      "Unknown byte at 11",
      "unknown-11",
      type = ftypes.NONE,
      range = { 11, 1 },
    },

    {
      "Monitor Mix stray bits???",
      "mix_stray_bits",
      type = ftypes.UINT8,
      range = { 12, 1 },
      base = base.DEC,
      desc = "Some weird unknown bits",
    },
    {
      "Monitor Mix",
      "mix",
      type = ftypes.UINT8,
      range = { 13, 1 },
      base = base.DEC,
      desc = "Mix between microphone and PC audio in %",
    },

    {
      "Unknown byte at 14",
      "unknown-14",
      type = ftypes.NONE,
      range = { 14, 1 },
    },

    {
      "Mute Color",
      "color_mute",
      type = ftypes.UINT24,
      range = { 15, 3 },
      base = base.HEX,
      decode = function(buf)
        return buf:range(0, 3):uint()
      end,
    },
    {
      "General Color",
      "color_gen",
      type = ftypes.UINT24,
      range = { 18, 9 },
      base = base.HEX,
      decode = function(buf)
        return buf:range(0, 3):uint()
      end,
      desc = "For some reason they appear *trice as part of the config bytes",
    },

    {
      "Unknown byte at 27",
      "unknown-27",
      type = ftypes.NONE,
      range = { 27, 1 },
    },

    {
      "Wave Gain Lock",
      "gain_lock",
      type = ftypes.BOOLEAN,
      range = { 28, 1 },
      base = 1,
      mask = 0x01,
    },
    {
      "Gain Reduction Color",
      "color_gain_reduction",
      type = ftypes.UINT24,
      range = { 29, 3 },
      base = base.HEX,
      decode = function(buf)
        return buf:range(0, 3):uint()
      end,
    },
    {
      "Clipguard Indicator",
      "clipguard_indicator",
      type = ftypes.BOOLEAN,
      range = { 32, 1 },
      base = 1,
      mask = 0x01,
    },
    {
      "Low Impedence Mode",
      "lim",
      type = ftypes.BOOLEAN,
      range = { 33, 1 },
    },
  },
}

local urb_type = Field.new("usb.urb_type")
local dir_in = Field.new("usb.transfer_flags.dir_in")

local elgato = Proto.new("usb.elgato", "Usb Elgato Wave XLR")
function elgato.dissector(buf, pinfo, tree)
  local type = urb_type().value
  local is_in = dir_in().value

  local URB_SUBMIT = 0x43
  local URB_COMPLETE = 0x53

  if type == URB_SUBMIT and is_in == false then
    header.dissector(buf:range(0, 7):tvb(), pinfo, tree)
    config.dissector(buf:range(7, 34):tvb(), pinfo, tree)
  elseif type == URB_COMPLETE and is_in == true then
    config.dissector(buf:range(0, 34):tvb(), pinfo, tree)
  end
end

DissectorTable.get("usb.control"):add(0xffff, elgato)
