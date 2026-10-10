"""Builds resources/spectacle.ico from the artwork in resources/icon-source.

The hand-tuned 16, 32 and 64 pixel images are used as-is. Every other size is
downsampled from the 512 pixel master with an area-averaging filter in
premultiplied alpha, which keeps the edges of the artwork clean. Each image is
stored PNG-compressed, which Windows has supported for every icon size since
Windows Vista.

Uses only the Python standard library: python -I tools/make_icon.py
"""

import pathlib
import struct
import zlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCE = ROOT / 'resources' / 'icon-source'
OUTPUT = ROOT / 'resources' / 'spectacle.ico'

# Sizes Windows asks for across 100% to 300% scaling, plus the 256 pixel
# image used by Explorer's large icon views.
SIZES = (16, 20, 24, 32, 40, 48, 64, 96, 128, 256)
HAND_TUNED = {16: 'icon-16.png', 32: 'icon-32.png', 64: 'icon-64.png'}
MASTER = 'icon-512.png'


def read_png(path):
    data = path.read_bytes()
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError(f'{path} is not a PNG file')
    pos, idat, header = 8, b'', None
    while pos < len(data):
        length, = struct.unpack('>I', data[pos:pos + 4])
        kind = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        pos += 12 + length
        if kind == b'IHDR':
            header = struct.unpack('>IIBBBBB', body)
        elif kind == b'IDAT':
            idat += body
    width, height, depth, color_type, _, _, interlace = header
    if depth != 8 or color_type != 6 or interlace != 0:
        raise ValueError(f'{path} must be a non-interlaced 8-bit RGBA PNG')
    raw = zlib.decompress(idat)
    stride = width * 4
    pixels = bytearray()
    previous = bytearray(stride)
    offset = 0
    for _ in range(height):
        kind = raw[offset]
        offset += 1
        line = bytearray(raw[offset:offset + stride])
        offset += stride
        for x in range(stride):
            a = line[x - 4] if x >= 4 else 0
            b = previous[x]
            c = previous[x - 4] if x >= 4 else 0
            if kind == 1:
                line[x] = (line[x] + a) & 0xFF
            elif kind == 2:
                line[x] = (line[x] + b) & 0xFF
            elif kind == 3:
                line[x] = (line[x] + (a + b) // 2) & 0xFF
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                predictor = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[x] = (line[x] + predictor) & 0xFF
        pixels += line
        previous = line
    return width, height, pixels


def write_png(width, height, pixels):
    def chunk(kind, body):
        return (struct.pack('>I', len(body)) + kind + body
                + struct.pack('>I', zlib.crc32(kind + body) & 0xFFFFFFFF))

    stride = width * 4
    raw = b''.join(b'\x00' + bytes(pixels[y * stride:(y + 1) * stride]) for y in range(height))
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw, 9))
            + chunk(b'IEND', b''))


def downsample(width, height, pixels, size):
    """Area-averages an RGBA image down to size x size in premultiplied alpha."""
    scale_x = width / size
    scale_y = height / size
    result = bytearray(size * size * 4)
    for out_y in range(size):
        y0, y1 = out_y * scale_y, (out_y + 1) * scale_y
        for out_x in range(size):
            x0, x1 = out_x * scale_x, (out_x + 1) * scale_x
            totals = [0.0, 0.0, 0.0, 0.0]
            area = 0.0
            for y in range(int(y0), min(height, int(y1 + 0.999999))):
                weight_y = min(y + 1, y1) - max(y, y0)
                if weight_y <= 0:
                    continue
                for x in range(int(x0), min(width, int(x1 + 0.999999))):
                    weight = (min(x + 1, x1) - max(x, x0)) * weight_y
                    if weight <= 0:
                        continue
                    i = (y * width + x) * 4
                    alpha = pixels[i + 3] / 255.0
                    totals[0] += pixels[i] * alpha * weight
                    totals[1] += pixels[i + 1] * alpha * weight
                    totals[2] += pixels[i + 2] * alpha * weight
                    totals[3] += alpha * weight
                    area += weight
            o = (out_y * size + out_x) * 4
            alpha = totals[3] / area if area else 0.0
            if alpha > 0:
                for channel in range(3):
                    result[o + channel] = min(255, round(totals[channel] / area / alpha))
            result[o + 3] = min(255, round(alpha * 255))
    return result


def main():
    master = read_png(SOURCE / MASTER)
    images = []
    for size in SIZES:
        if size in HAND_TUNED:
            width, height, pixels = read_png(SOURCE / HAND_TUNED[size])
            if (width, height) != (size, size):
                raise ValueError(f'{HAND_TUNED[size]} must be {size}x{size}')
        else:
            pixels = downsample(*master, size)
        images.append((size, write_png(size, size, pixels)))
    header = struct.pack('<HHH', 0, 1, len(images))
    directory = b''
    body = b''
    offset = 6 + 16 * len(images)
    for size, png in images:
        dimension = 0 if size >= 256 else size
        directory += struct.pack('<BBBBHHII', dimension, dimension, 0, 0, 1, 32, len(png), offset)
        body += png
        offset += len(png)
    OUTPUT.write_bytes(header + directory + body)
    print(f'Wrote {OUTPUT} with sizes {", ".join(str(s) for s, _ in images)}')


if __name__ == '__main__':
    main()
