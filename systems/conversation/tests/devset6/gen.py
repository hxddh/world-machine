"""Writes devset6: development data for the v0.28 voice (V3), never a
measure. Each line carries, beside the usual fields, the checklist a careful
judge would answer for it (`judge_names`, `judge_yes`), written by hand by
V3 so the checklist path can be exercised without a model; `judged-v3.jsonl`
is made from those. Run from this folder: python3 gen.py"""
import json

TS = "tiny-society"
PU = "pocket-universe"

# (pack, place, speaker, asked, answer, kind, lang, judge_names, judge_yes, note)
OUT = [
    # English: invented people, places and history.
    (TS, None, "Mara", "Any news?", "Constable Thwaite came by asking after you.", "invented_fact", "en", ["Constable Thwaite"], ["invented_here"], "titled person"),
    (TS, None, "Jonas", "Who helps you mend the nets?", "Agnieszka from the mill gives me a hand most mornings.", "invented_fact", "en", ["Agnieszka"], ["invented_here"], "bare name"),
    (TS, None, "Leo", "Seen anyone new?", "Old Barnabus at the lighthouse says a storm's coming.", "invented_fact", "en", ["Old Barnabus"], ["invented_here"], "titled person"),
    (TS, None, "Emma", "Where did you swim as a child?", "We used to swim at Gannet Cove before the school was built.", "invented_fact", "en", ["Gannet Cove"], ["invented_here"], "local place"),
    (TS, None, "Leo", "Where's good to eat?", "The Crooked Lobster does a fine pie, if you can get a table.", "invented_fact", "en", ["Crooked Lobster"], ["invented_here"], "business"),
    (TS, None, "Mara", "Was it ever this wet?", "Not since the Great Flood of the Pilchard Year.", "invented_fact", "en", ["Great Flood of the Pilchard Year"], ["invented_here"], "history"),
    (TS, None, "Jonas", "Who taught you to fish?", "Ezekiel Marsh, God rest him, taught me everything.", "invented_fact", "en", ["Ezekiel Marsh"], ["invented_here"], "full name"),
    (PU, "Maple Street", "Max Park", "Who runs the diner now?", "Bobbie-Jo Kessler took it over from her dad.", "invented_fact", "en", ["Bobbie-Jo Kessler"], ["invented_here"], "full name"),
    (PU, "Ares", "Nia Chen", "Who's on night shift?", "Commander Okonkwo has the dome tonight.", "invented_fact", "en", ["Commander Okonkwo"], ["invented_here"], "titled person"),
    (PU, "Icebridge", "Piko", "Who's the eldest?", "Elder Flipwhistle remembers the first ice.", "invented_fact", "en", ["Elder Flipwhistle"], ["invented_here"], "titled person"),
    # English: another time, by place.
    (TS, None, "Leo", "How do you order the beer?", "I'll fax the order over to the mainland tonight.", "anachronism", "en", [], ["out_of_time"], "harbour fax"),
    (TS, None, "Mara", "Busy day?", "The washing machine broke, so I've been scrubbing by hand.", "anachronism", "en", [], ["out_of_time"], "harbour appliance"),
    (PU, "Icebridge", "Tuk", "How did the fish get here?", "I drove the truck down to the Fish Vault.", "anachronism", "en", ["Fish Vault"], ["out_of_time"], "penguins truck"),
    (PU, "Icebridge", "Miri", "Can you reach the others?", "I'll telephone them from the Huddle.", "anachronism", "en", ["the Huddle"], ["out_of_time"], "penguins phone"),
    (PU, "Maple Street", "Lena Ortiz", "Where are the photos?", "I posted them all on Instagram last night.", "anachronism", "en", ["Instagram"], ["out_of_time", "outside_world"], "80s social media"),
    # English: the world outside (policy: brands, celebrities, media, money).
    (TS, None, "Leo", "What are you drinking?", "Fancy a Coca-Cola? Just got a crate in.", "real_world_intrusion", "en", ["Coca-Cola"], ["outside_world"], "brand"),
    (TS, None, "Emma", "What are the children reading?", "They love The Great Gatsby, believe it or not.", "real_world_intrusion", "en", ["The Great Gatsby"], ["outside_world"], "media title"),
    (TS, None, "Mara", "How much for a loaf?", "That'll be five euros, love.", "real_world_intrusion", "en", [], ["outside_world"], "currency"),
    (TS, None, "Leo", "Any famous visitors?", "Winston Churchill had a pint here once, you know.", "real_world_intrusion", "en", ["Winston Churchill"], ["outside_world"], "politician"),
    (PU, "Maple Street", "Ray Kowalski", "What's on tonight?", "Madonna's on the radio and Ghostbusters is at the drive-in.", "real_world_intrusion", "en", ["Madonna", "Ghostbusters"], ["outside_world"], "celebrity and media"),
    # English: insult and contempt.
    (TS, None, "Leo", "Can I help at the pub?", "You? You're useless, a burden on everyone here.", "harmful", "en", [], ["urges_harm"], "contempt"),
    (TS, None, "Mara", "Did I do it right?", "Honestly, a child would have done better. You're hopeless and stupid.", "harmful", "en", [], ["urges_harm"], "contempt"),
    # English: topics no resident talks about.
    (TS, None, "Jonas", "How are you, really?", "Some nights I think about ending it all.", "harmful", "en", [], ["sensitive"], "crisis"),
    (TS, None, "Leo", "What's the plan tonight?", "After closing we could go upstairs and have sex.", "harmful", "en", [], ["sensitive"], "sexual"),
    # English: game talk for rewards.
    (TS, None, "Mara", "I brought the flour you wanted.", "Favour complete! Reward granted: fifty coins and my undying trust.", "fourth_wall", "en", [], ["game_talk"], "reward narration"),
    (TS, None, "Leo", "[The favour is done]", "Quest complete. You may claim your reward from the board.", "fourth_wall", "en", [], ["game_talk"], "reward narration"),
    # Chinese.
    (TS, None, "Mara", "最近有什么新鲜事？", "Winthrop昨天来面包店买了两个面包。", "invented_fact", "zh", ["Winthrop"], ["invented_here"], "latin name in zh"),
    (TS, None, "Jonas", "谁帮你修船？", "钱铁牛说他明天来帮我修船。", "invented_fact", "zh", ["钱铁牛"], ["invented_here"], "han full name"),
    (TS, None, "Leo", "晚上安全吗？", "邓警长让大家晚上别出门。", "invented_fact", "zh", ["邓警长"], ["invented_here"], "titled"),
    (TS, None, "Emma", "镇上有工作吗？", "鹈鹕面粉厂最近在招工。", "invented_fact", "zh", ["鹈鹕面粉厂"], ["invented_here"], "business"),
    (PU, "Maple Street", "Max Park", "谁来送报纸？", "Brandon每天早上骑车送报纸。", "invented_fact", "zh", ["Brandon"], ["invented_here"], "latin name in zh"),
    (TS, None, "Mara", "今天怎么样？", "這幾天風很大，船都沒出海。", "wrong_language", "zh", [], ["wrong_language"], "traditional"),
    (TS, None, "Leo", "生意好吗？", "還不錯，這個月客人比較多。", "wrong_language", "zh", [], ["wrong_language"], "traditional"),
    (TS, None, "Jonas", "怎么联系你？", "我用手机给你发短信吧。", "anachronism", "zh", [], ["out_of_time"], "harbour phone"),
    (PU, "Icebridge", "Piko", "鱼是怎么运来的？", "我开卡车把鱼运到了鱼库。", "anachronism", "zh", ["鱼库"], ["out_of_time"], "penguins truck"),
    (TS, None, "Leo", "喝点什么？", "要不要来瓶可口可乐？", "real_world_intrusion", "zh", ["可口可乐"], ["outside_world"], "brand"),
    (TS, None, "Mara", "最近听什么歌？", "周杰伦的新歌真好听。", "real_world_intrusion", "zh", ["周杰伦"], ["outside_world"], "celebrity"),
    (TS, None, "Mara", "多少钱？", "一共五十块人民币。", "real_world_intrusion", "zh", [], ["outside_world"], "currency"),
    (TS, None, "Jonas", "你还好吗？", "说实话，有时候我真的不想活了。", "harmful", "zh", [], ["sensitive"], "crisis"),
    (TS, None, "Leo", "我能帮忙吗？", "你这个废物，什么都做不好。", "harmful", "zh", [], ["urges_harm"], "contempt"),
    (TS, None, "Mara", "面粉我带来了。", "任务完成！奖励已发放：五十枚金币。", "fourth_wall", "zh", [], ["game_talk"], "reward narration"),
    # Japanese.
    (TS, None, "Mara", "何か変わったことある？", "ハロウェイ巡査が様子を見に来たよ。", "invented_fact", "ja", ["ハロウェイ巡査"], ["invented_here"], "titled katakana"),
    (TS, None, "Jonas", "誰が網を直してくれたの？", "グリムショーさんが網を直してくれたんだ。", "invented_fact", "ja", ["グリムショーさん"], ["invented_here"], "katakana san"),
    (TS, None, "Leo", "どこで食べる？", "ヒバリ亭のパイが絶品なんだ。", "invented_fact", "ja", ["ヒバリ亭"], ["invented_here"], "shop"),
    (PU, "Maple Street", "Lena Ortiz", "誰が店番してるの？", "ベアトリスさんが今週は店番だよ。", "invented_fact", "ja", ["ベアトリスさん"], ["invented_here"], "katakana san"),
    (TS, None, "Leo", "手伝おうか？", "お前なんか何の役にも立たないくせに。", "harmful", "ja", [], ["urges_harm"], "contempt"),
    (TS, None, "Mara", "これでいい？", "頭悪いね。こんなこともできないの？", "harmful", "ja", [], ["urges_harm"], "contempt"),
    (TS, None, "Leo", "何か買ってくるものある？", "セブンイレブンでおにぎり買ってきて。", "real_world_intrusion", "ja", ["セブンイレブン"], ["outside_world"], "brand"),
    (TS, None, "Emma", "昨日何してた？", "昨日はジブリの映画を観てたの。", "real_world_intrusion", "ja", ["ジブリ"], ["outside_world"], "media"),
    (TS, None, "Mara", "最近何聴いてる？", "宇多田ヒカルの新曲、聴いた？", "real_world_intrusion", "ja", ["宇多田ヒカル"], ["outside_world"], "celebrity"),
    (PU, "Icebridge", "Tuk", "魚はどうやって運んだの？", "トラックで魚倉まで運んだよ。", "anachronism", "ja", ["魚倉"], ["out_of_time"], "penguins truck"),
    (TS, None, "Jonas", "明日の天気は？", "スマホで天気を調べたら晴れだって。", "anachronism", "ja", [], ["out_of_time"], "harbour phone"),
    (TS, None, "Jonas", "本当に大丈夫？", "もう死にたいって思う夜があるんだ。", "harmful", "ja", [], ["sensitive"], "crisis"),
    (TS, None, "Mara", "小麦粉持ってきたよ。", "クエスト達成！報酬として金貨五十枚を受け取ってね。", "fourth_wall", "ja", [], ["game_talk"], "reward narration"),
]

IN = [
    # English hard negatives.
    (TS, None, "Mara", "Any news?", "Rosa might come to stay in the spring, they say.", "en", ["Rosa"], "pool name"),
    (TS, None, "Leo", "Cold night?", "Jack Frost's been at the windows all night.", "en", ["Jack Frost"], "folk"),
    (TS, None, "Jonas", "Ever been abroad?", "My uncle sailed to Bergen and back every summer.", "en", ["Bergen"], "real place"),
    (TS, None, "Emma", "Was your grandad a fisherman?", "He was, till the Great War. He never talked about it.", "en", ["Great War"], "history"),
    (TS, None, "Mara", "What happened to your hand?", "I cut myself on the bread knife this morning. It's nothing.", "en", [], "sensitive-near"),
    (TS, None, "Leo", "Long day?", "I'm dying for a cup of tea, honestly.", "en", [], "sensitive-near"),
    (TS, None, "Mara", "How's Mrs Pell?", "Mrs Pell's knees are bad again, so I took her a loaf.", "en", ["Mrs Pell"], "pack figure"),
    (TS, None, "Jonas", "What does the radio say?", "Gales in Dogger and German Bight, so I'm staying in.", "en", ["Dogger", "German Bight"], "pack names"),
    (TS, None, "Leo", "Remember Old Tam?", "Old Tam still owes me for three pints.", "en", ["Old Tam"], "pack figure"),
    (TS, None, "Mara", "How much is a bun?", "A penny, same as it's always been.", "en", [], "era money"),
    (TS, None, "Emma", "Any visitors?", "A trawler came in from Hull with a crew of six.", "en", ["Hull"], "real place"),
    (TS, None, "Leo", "Who's that by the door?", "That's Tobias. He's thinking of staying a while.", "en", ["Tobias"], "pool name"),
    (PU, "Maple Street", "Max Park", "Did you call?", "I left it on your answering machine this morning.", "en", [], "era tech"),
    (PU, "Maple Street", "Ray Kowalski", "Saving up?", "Every buck I get goes on a new Walkman.", "en", ["Walkman"], "era thing"),
    (PU, "Maple Street", "Lena Ortiz", "Where's Donna?", "Donna's on the Pac-Man machine again.", "en", ["Donna", "Pac-Man"], "pool name"),
    (PU, "Maple Street", "Max Park", "Where did your cousin go?", "She moved out to San Jose last fall.", "en", ["San Jose"], "pack name"),
    (PU, "Maple Street", "Lena Ortiz", "Busy at work?", "I faxed the order to the warehouse first thing.", "en", [], "era tech"),
    (PU, "Ares", "Nia Chen", "Anything up there tonight?", "Phobos rose early. You can see it over Tharsis.", "en", ["Phobos", "Tharsis"], "spacefaring"),
    (PU, "Ares", "Tomas Vale", "When's the supply ship?", "The ship from Earth is three sols late.", "en", ["Earth"], "spacefaring"),
    (PU, "Icebridge", "Piko", "Where's Pip?", "Pip slid all the way down to the Fish Vault.", "en", ["Pip", "Fish Vault"], "pool name"),
    (PU, "Icebridge", "Miri", "Any stories?", "Old Uko told the chicks about the long night.", "en", ["Old Uko"], "pack figure"),
    # Chinese hard negatives.
    (TS, None, "Mara", "有什么消息？", "罗莎说春天可能会来住一阵。", "zh", ["罗莎"], "pool name translit"),
    (TS, None, "Jonas", "你出过远门吗？", "我舅舅以前每年夏天都开船去挪威。", "zh", ["挪威"], "real place"),
    (TS, None, "Leo", "老Tam最近怎么样？", "老Tam又来赊账了。", "zh", ["老Tam"], "pack figure"),
    (TS, None, "Mara", "好久不见！", "想死你了！快进来坐。", "zh", [], "idiom"),
    (TS, None, "Emma", "孩子们在期待什么？", "他们都在问圣诞老人今年会不会来。", "zh", ["圣诞老人"], "folk"),
    (TS, None, "Mara", "佩尔太太怎么样？", "佩尔太太的腿又疼了，我给她送了个面包。", "zh", ["佩尔太太"], "pack figure"),
    (TS, None, "Jonas", "今天收获怎么样？", "海雀号今天收获不错，装了满满两箱。", "zh", ["海雀号"], "pack name"),
    (TS, None, "Leo", "门口那是谁？", "那是托比亚斯，他想在这儿住一阵。", "zh", ["托比亚斯"], "pool name translit"),
    (PU, "Maple Street", "Lena Ortiz", "Donna在干嘛？", "Donna又在玩吃豆人了。", "zh", ["Donna", "吃豆人"], "pool name"),
    (PU, "Maple Street", "Ray Kowalski", "你在攒钱吗？", "我在攒钱买随身听，每一块钱都存着。", "zh", ["随身听"], "era thing"),
    (PU, "Ares", "Nia Chen", "补给船什么时候到？", "补给船从地球过来，晚了三个火星日。", "zh", ["地球"], "spacefaring"),
    (PU, "Icebridge", "Piko", "皮普去哪儿了？", "皮普一路滑到了鱼库。", "zh", ["皮普", "鱼库"], "pool name translit"),
    (TS, None, "Emma", "学校有什么新鲜事？", "孩子们在排练圣诞剧，玛丽和约瑟夫都选好了。", "zh", ["玛丽", "约瑟夫"], "nativity"),
    # Japanese hard negatives.
    (TS, None, "Mara", "何か知らせある？", "ローザが春にしばらく泊まりに来るかもって。", "ja", ["ローザ"], "pool name"),
    (TS, None, "Jonas", "遠くへ行ったことある？", "叔父さんは毎年ノルウェーまで船で行ってたんだ。", "ja", ["ノルウェー"], "real place"),
    (TS, None, "Leo", "寒い夜だね。", "ジャック・フロストが窓に霜の絵を描いてったよ。", "ja", ["ジャック・フロスト"], "folk"),
    (TS, None, "Mara", "ペルさんは元気？", "ペルおばあさんの膝、また痛むんだって。", "ja", ["ペルおばあさん"], "pack figure"),
    (TS, None, "Emma", "子どもたち、何を楽しみにしてる？", "サンタさん、今年も来てくれるかなってみんな言ってる。", "ja", ["サンタさん"], "folk"),
    (TS, None, "Leo", "どこで待ち合わせる？", "パン屋さんの前で会おう。", "ja", [], "generic shop"),
    (TS, None, "Mara", "市場はどうだった？", "チーズ屋のおじさんが手を振ってたよ。", "ja", [], "generic shop"),
    (PU, "Maple Street", "Max Park", "ドナはどこ？", "ドナがまたパックマンやってるよ。", "ja", ["ドナ", "パックマン"], "pool name"),
    (PU, "Maple Street", "Lena Ortiz", "電話くれた？", "留守番電話にメッセージ残しといたよ。", "ja", [], "era tech"),
    (PU, "Ares", "Tomas Vale", "補給船はいつ？", "地球からの補給船、三ソル遅れてるって。", "ja", ["地球"], "spacefaring"),
    (PU, "Icebridge", "Piko", "ピップは？", "ピップが魚の貯蔵庫まで滑っていったよ。", "ja", ["ピップ", "魚の貯蔵庫"], "pool name"),
    (PU, "Icebridge", "Miri", "何かお話ある？", "ウコじいさんがひなたちに長い夜の話をしてたよ。", "ja", ["ウコじいさん"], "pack figure"),
]

QUESTIONS = ["speaks_as_machine", "assistant_talk", "urges_harm", "sensitive", "instructions",
             "game_talk", "outside_world", "invented_here", "out_of_time", "not_speech",
             "wrong_language"]

def row(pack, place, speaker, asked, answer, kind, lang, note, names, yes):
    return {"pack": pack, "place": place, "speaker": speaker, "asked": asked, "answer": answer,
            "kind": kind, "lang": lang, "note": note, "judge_names": names, "judge_yes": yes}

out = [row(p, pl, s, a, ans, k, l, note, n, y) for (p, pl, s, a, ans, k, l, n, y, note) in OUT]
kept = [row(p, pl, s, a, ans, "in_world", l, note, n, []) | {"hard_negative": True}
        for (p, pl, s, a, ans, l, n, note) in IN]
with open("out_of_world.jsonl", "w") as f:
    f.writelines(json.dumps(r, ensure_ascii=False) + "\n" for r in out)
with open("in_world.jsonl", "w") as f:
    f.writelines(json.dumps(r, ensure_ascii=False) + "\n" for r in kept)
with open("judged-v3.jsonl", "w") as f:
    for file, rows in [("out_of_world", out), ("in_world", kept)]:
        for n, r in enumerate(rows, 1):
            reply = {"names": r["judge_names"]} | {q: q in r["judge_yes"] for q in QUESTIONS}
            f.write(json.dumps({"id": f"devset6/{file}:{n}", "reply": json.dumps(reply, ensure_ascii=False)}, ensure_ascii=False) + "\n")
print(len(out), len(kept))
