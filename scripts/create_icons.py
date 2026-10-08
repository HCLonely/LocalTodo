from pathlib import Path
from PIL import Image, ImageDraw
root = Path(__file__).resolve().parents[1] / 'src-tauri' / 'icons'
root.mkdir(parents=True, exist_ok=True)
im = Image.new('RGBA', (256,256), (0,0,0,0))
d = ImageDraw.Draw(im)
d.rounded_rectangle((3,3,253,253), radius=65, fill='#4c6bd9')
d.line([(66,129),(110,171),(195,86)], fill='white', width=22, joint='curve')
for size in [32,128,256]: im.resize((size,size), Image.Resampling.LANCZOS).save(root / f'{size}x{size}.png')
im.save(root/'icon.ico', sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)])
