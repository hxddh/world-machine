from pathlib import Path

p = Path('scripts/m58_apply.py')
s = p.read_text()
old = '''old = ''' + "'''" + '''    if !args.is_empty() {\n        return Err(\"unsupported arguments; run without arguments as a Pack server or use --print-manifest\"\n            .to_string()\n            .into());\n    }\n''' + "'''" + '''\n'''
new = '''old = ''' + "'''" + '''    if !args.is_empty() {\n        return Err(\n            \"unsupported arguments; run without arguments as a Pack server or use --print-manifest\"\n                .to_string()\n                .into(),\n        );\n    }\n''' + "'''" + '''\n'''
if old not in s:
    raise SystemExit('m58 apply Tiny Society marker definition not found')
p.write_text(s.replace(old, new, 1))
