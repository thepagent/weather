# weather

A Rust CLI that fetches current weather from [Open-Meteo](https://open-meteo.com/) and optionally generates an AI image via the Gemini API — no external binaries required.

## Usage

```bash
weather --timezone Asia/Taipei
# weather=Partly cloudy current=13.9°C max=20.9°C min=13.8°C localtime=2026-03-14T02:15
```

Any IANA timezone is supported — city coordinates are resolved automatically via the Open-Meteo Geocoding API:

```bash
weather --timezone Asia/Keelung
weather --timezone Asia/Tainan
weather --timezone Europe/Paris
weather --timezone America/Chicago
```

### Generate image

```bash
GEMINI_API_KEY=<key> weather --timezone Asia/Shanghai \
  --image \
  --prompt "夢幻插畫風格，手機桌布" \
  --output /tmp/weather.png
```

### Override model

```bash
GEMINI_API_KEY=<key> weather --timezone Asia/Taipei \
  --image \
  --model gemini-3.2-flash-image-preview \
  --prompt "dreamy illustration, no text" \
  --output /tmp/out.png
```

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--timezone <tz>` | Any IANA timezone | `America/New_York` |
| `--image` | Generate weather image via Gemini API | off |
| `--model <id>` | Gemini model ID | `gemini-3.1-flash-image-preview` |
| `--prompt <text>` | Extra prompt appended to image generation | — |
| `--output <path>` | Image output path | `output.png` |
| `--resolution <res>` | `1K`, `2K`, `4K` (reserved) | `1K` |

## Image generation

Requires `GEMINI_API_KEY` set in the environment. The prompt sent to Gemini is auto-constructed as:

```
{city}, {localtime}, {weather}, current {temp}°C, max {max}°C, min {min}°C. {--prompt}
```

## Install

```bash
cargo build --release
ln -sf $PWD/target/release/weather ~/.local/bin/weather
```

## Sample for OpenClaw

```
Execute this exact command to generate the image:

weather --timezone America/New_York --image --prompt "根據所給的時間、城市與天氣進行創作，夢幻插畫風格，手機桌布，展現城市、建築、交通、地標、美食、人文、風景、文化等隨機其中一個特色，畫面元素不要太多，所有文字都不要" --output /path-to-my-workspace/outputs/nyc-weather-latest.png

Then send this exact image file to my Telegram with force-document by EXECUTING this exact command:

openclaw message send --channel telegram --account <MY_ACCOUNT> --target <MY_TG_ID> --media /path-to-my-workspace/outputs/nyc-weather-latest.png --force-document -m "<TRADITIONAL_CHINESE_CAPTION>"
```
