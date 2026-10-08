# Store description: the handmade town first

The short description for Steam (at most 300 characters) and the opening lines of the long one, in English, Simplified Chinese and Japanese. They lead with the town as it looks: a small place of cut paper and paint on a table, drawn by the game's own code. The voice is not in the short description; the game and its demo are whole without it. The longer text, the features and the system requirements stay in [../store-page.md](../store-page.md), which this replaces at the top.

Checked against the code at `v0.28` (see "Where each claim comes from" below). Leave out any sentence that stops being true.

---

## English

### Short description (264 characters)

A little harbour town of cut paper and paint that goes on living while you are away. Answer what its eight neighbours ask, build with your own hands, then close the window. When you come back, the town shows you what happened, and it is the town your answers made.

### Opening of the long description

**A small town on your table, that carries on without you.**

The harbour is a diorama of cut paper and gouache, lit by your Mac's own clock: morning light on the quay, lamps along the water at dusk, windows that tell you who is home at night. Everything in it stands where it belongs and at the size of the people who use it, and everything you build stays there, in its own corner of the town.

---

## 简体中文

### 简短描述（89 字）

一座纸艺与水彩般的小小海港，你不在的时候也照样过日子。回答八位邻居的请求，亲手建造，然后关上窗口。等你回来，小镇会让你看到这段时间发生了什么——而这座小镇，正是由你的回答塑造的。

### 详细描述的开头

**一座放在桌上的小镇，没有你也照样过下去。**

海港像一座用剪纸和水粉做成的微缩景观，光线随你 Mac 的时钟变化：清晨照在码头上的阳光，黄昏时沿着水边亮起的路灯，夜里亮着灯的窗户告诉你谁在家。镇上的每样东西都待在该在的地方，大小和使用它的人相称；你亲手建造的东西也会一直留在那里，留在小镇属于它的角落。

---

## 日本語

### 短い説明（128 文字）

あなたが留守のあいだも暮らしが続く、切り紙と絵の具でできたような小さな港町。8 人の住人の頼みごとに答え、自分の手でものを作ったら、ウィンドウを閉じましょう。戻ってくると、町は留守中の出来事を見せてくれます。そこにあるのは、あなたの答えが形づくった町です。

### 詳しい説明の書き出し

**テーブルの上の小さな町。あなたがいなくても、暮らしは続いていきます。**

港町は、切り紙とガッシュでできたジオラマのよう。光は Mac の時計に合わせて移ろいます。朝の波止場にさす日ざし、夕暮れに水辺に灯るランプ、夜には窓の明かりで誰が家にいるのかがわかります。町のものはどれも、あるべき場所に、使う人に見合った大きさで立っていて、あなたが作ったものは、町の自分の一角にずっと残ります。

---

## Where each claim comes from

| Claim | Where it is held |
|---|---|
| Cut paper and paint, drawn by the game's own code | [ART_DIRECTION.md](../../ART_DIRECTION.md) ("a lit paper-and-paint diorama"); the drawings are code, not an illustrator's ([KNOWN_ISSUES](../../KNOWN_ISSUES.md)) |
| Light follows the Mac's clock; lamps at dusk; lit windows at night | KNOWN_ISSUES ("The sky follows your clock"); `world-gpui` diorama tests for lamps and windows |
| Everything at human scale, where it belongs | `the_bibles_ladder_holds`, `water_works_stand_on_the_water_line` |
| What you build stays, in its own corner | the named clusters (`days::town`), scaffolding on its own site from the first answer |
| Eight neighbours | Tiny Society's core residents (KNOWN_ISSUES: "Tiny Society's eight") |
| Goes on living while you are away | a day for every six hours, up to a week (KNOWN_ISSUES, "Background time is bounded") |
| Shows you what happened | the return film |

**Character counts** are counted as Steam counts them (characters, not bytes), and each short description is under 300. The Chinese and Japanese were written by a model and have not been read by a native speaker; have each read before the page is public.
