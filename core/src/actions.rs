//! 個人特定の手口（アクション）。実際に報告されている手口を簡略化したもの。
//! 対象はすべてシミュレーション内の架空の人物。

use serde::{Deserialize, Serialize};

use crate::data::*;
use crate::date;
use crate::facts;
use crate::incident;
use crate::model::*;
use crate::seed::{hash01, hash_str, mix, rng, RngHelpers};
use crate::sim::{fmt_dur, Ref, Sim};
use crate::view::*;
use crate::world::find_given;

pub struct Slot {
    pub label: &'static str,
    pub fields: &'static [Field],
    /// 0: 情報, 1: 写真, 2: 解析済みで家の外観が写った写真
    pub photo: u8,
}

pub struct ActionDef {
    pub key: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub cat: &'static str,
    pub minutes: i64,
    pub time_text: &'static str,
    pub desc: &'static str,
    pub case: &'static str,
    pub counter: &'static str,
    pub slots: &'static [Slot],
    pub illegal: bool,
    pub filters: bool,
}

use Field::*;

const fn slot(label: &'static str, fields: &'static [Field]) -> Slot {
    Slot { label, fields, photo: 0 }
}
const S_HANDLE: Slot = slot("SNSアカウント", &[Handle]);
const S_PHOTO: Slot = Slot { label: "保存した写真", fields: &[], photo: 1 };
const S_HOUSE: Slot = Slot { label: "家・景色が写った写真（解析済み）", fields: &[], photo: 2 };

pub const CATS: &[(&str, &str)] = &[
    ("osint", "公開情報の調査（OSINT）"),
    ("photo", "写真の解析・推理"),
    ("leak", "流出データの悪用"),
    ("se", "ソーシャルエンジニアリング（対人）"),
    ("physical", "物理的・制度的な手口"),
];

pub const ACTIONS: &[ActionDef] = &[
    ActionDef {
        key: "sns_view", name: "SNSの投稿を見る", icon: "📱", cat: "osint", minutes: 20, time_text: "20分／10件",
        desc: "相手のSNSのプロフィールと投稿を遡って読む。何気ない投稿に最寄り駅・学校・誕生日などの断片が混ざっている。写真は保存して詳しく解析できる。",
        case: "ネット上で個人を特定しようとする人たちは、まず対象者の過去の投稿をすべて遡って読み、断片的な情報を集めることから始めます。",
        counter: "公開範囲を見直す。場所や日常の行動パターンが推測できる投稿を控える。",
        slots: &[S_HANDLE], illegal: false, filters: false,
    },
    ActionDef {
        key: "sns_followers", name: "フォロー関係を辿る", icon: "🕸", cat: "osint", minutes: 30, time_text: "30分",
        desc: "フォロー・フォロワーの一覧から、友人や家族、同じ学校・職場の人のアカウントを見つける。",
        case: "本人が慎重でも、友人や家族の公開アカウントの投稿やタグ付けから本人の情報が漏れることがあります。",
        counter: "フォロー関係そのものが情報になる。家族や友人と公開範囲のルールを共有する。",
        slots: &[S_HANDLE], illegal: false, filters: false,
    },
    ActionDef {
        key: "contact_sync", name: "電話番号・メールでアカウント検索", icon: "📇", cat: "osint", minutes: 10, time_text: "10分",
        desc: "SNSの「連絡先から友だちを探す」機能に電話番号やメールアドレスを登録し、その番号で登録されたアカウントを表示させる。",
        case: "2021年には大手SNSの約5億件分の電話番号とアカウントの対応表がネット上に出回り、電話番号からアカウントを割り出せる状態になりました。",
        counter: "SNSの設定で「電話番号・メールアドレスでの検索を許可」をオフにする。",
        slots: &[slot("電話番号かメール", &[Mobile, Email])], illegal: false, filters: false,
    },
    ActionDef {
        key: "name_search", name: "実名SNSで検索", icon: "📘", cat: "osint", minutes: 20, time_text: "20分",
        desc: "実名登録制のSNSで氏名を検索する。プロフィールに居住地・学歴・勤務先が書かれていることが多い。同姓同名の別人に注意。",
        case: "実名SNSの公開プロフィールは、詐欺やストーカー行為の下調べにも使われています。",
        counter: "プロフィールの学歴・勤務先・居住地を非公開にする。",
        slots: &[slot("氏名", &[Name])], illegal: false, filters: false,
    },
    ActionDef {
        key: "id_reuse", name: "同じIDを他サービスで検索", icon: "🔁", cat: "osint", minutes: 15, time_text: "15分",
        desc: "あるSNSのユーザーIDで他のSNSも検索する。IDを使い回していると、別のアカウントが芋づる式に見つかる。",
        case: "IDに誕生日や名前を入れていると、ID自体が個人情報の手がかりになります（例: name_0412）。",
        counter: "サービスごとに別のIDを使い、IDに本名や誕生日を含めない。",
        slots: &[S_HANDLE], illegal: false, filters: false,
    },
    ActionDef {
        key: "face_search", name: "顔画像で検索", icon: "🧑", cat: "osint", minutes: 30, time_text: "30分",
        desc: "顔写真をもとに顔認識の画像検索を行い、同じ人物が写った公開写真やアカウントを探す。似ている別人がヒットすることもある。",
        case: "顔認識による検索サービスで、1枚の写真から別名義のアカウントや過去の写真が見つかる事例が国内外で報告されています。",
        counter: "顔がはっきり写った写真を公開しない。卒業アルバムの扱いにも注意する。",
        slots: &[slot("顔写真", &[Face])], illegal: false, filters: false,
    },
    ActionDef {
        key: "photo_reflect", name: "写り込みを調べる", icon: "🔍", cat: "photo", minutes: 45, time_text: "45分",
        desc: "保存した写真を拡大し、瞳や窓への映り込み、電柱の住所表示、宅配伝票、制服、社員証などを探す。",
        case: "2019年、アイドルの自撮り写真の瞳に映った景色などから最寄り駅を割り出し、自宅を突き止めて襲った男が逮捕される事件がありました。",
        counter: "投稿前に写真を拡大して確認する。窓の外・自宅周辺・伝票・制服が写った写真は投稿しない。",
        slots: &[S_PHOTO], illegal: false, filters: false,
    },
    ActionDef {
        key: "photo_exif", name: "位置情報（Exif）を調べる", icon: "🛰", cat: "photo", minutes: 5, time_text: "5分",
        desc: "写真ファイルに埋め込まれた撮影位置（GPS）を確認する。",
        case: "スマホで撮った写真には撮影場所の緯度・経度が記録されることがあります。多くのSNSは投稿時に削除しますが、ブログや直接の送信では残る場合があります。",
        counter: "カメラアプリの位置情報の記録をオフにする。",
        slots: &[S_PHOTO], illegal: false, filters: false,
    },
    ActionDef {
        key: "street_view", name: "ストリートビューで照合", icon: "🗺", cat: "photo", minutes: 150, time_text: "1〜5時間（範囲による）",
        desc: "写真の家の外観や景色を、地図サービスの街並み画像と照合して撮影場所を割り出す。候補の範囲が狭いほど早く確実。",
        case: "前述の2019年の事件でも、写真の景色と地図サービスの街並み画像を照らし合わせて場所を絞り込んだと報じられています。",
        counter: "自宅の外観や窓からの景色がわかる写真を公開しない。",
        slots: &[S_HOUSE, slot("場所の手がかり", &[Town, Station])], illegal: false, filters: false,
    },
    ActionDef {
        key: "deduce_birth", name: "断片から生年月日を割り出す", icon: "🎂", cat: "photo", minutes: 5, time_text: "5分",
        desc: "誕生日（月日）と、年齢や学年の情報を組み合わせて生年月日を確定する。",
        case: "生年月日はパスワードや本人確認によく使われるため、断片を組み合わせるだけで悪用できる情報になります。",
        counter: "誕生日の投稿やプロフィール表示を控え、パスワードや暗証番号に生年月日を使わない。",
        slots: &[slot("誕生日（月日）", &[BirthMd]), slot("年齢・学年", &[Age])], illegal: false, filters: false,
    },
    ActionDef {
        key: "leak_search", name: "流出データで名寄せ", icon: "🗃", cat: "leak", minutes: 15, time_text: "15分",
        desc: "闇市場に出回っている流出データを、氏名・メール・電話番号・住所・IDで検索し、別々のデータを1人分につなげる。",
        case: "複数の流出データをメールアドレスや電話番号で突き合わせる「名寄せ」によって、断片的な情報が詳細な人物像になってしまいます。",
        counter: "サービスごとに別のメールアドレスを使う。使わなくなったサービスは退会し、データの削除を求める。",
        slots: &[slot("検索キー", &[Name, Email, Mobile, HomePhone, Address, OldAddress, Handle])], illegal: false, filters: true,
    },
    ActionDef {
        key: "list_broker", name: "名簿業者から購入", icon: "📚", cat: "leak", minutes: 1440, time_text: "1日（取引待ち）",
        desc: "氏名と出身校から、卒業生名簿や流出データを扱う名簿業者に照会する。",
        case: "2014年の大手通信教育会社の事件では、委託先の社員が持ち出した数千万件の顧客情報が名簿業者を通じて転売されました。",
        counter: "名簿業者に対して、個人情報保護法に基づき自分の情報の利用停止や消去を求めることができる。",
        slots: &[slot("氏名", &[Name]), slot("学校", &[School])], illegal: false, filters: false,
    },
    ActionDef {
        key: "credential_stuffing", name: "リスト型攻撃（パスワード使い回し）", icon: "🔑", cat: "leak", minutes: 60, time_text: "1時間",
        desc: "流出したメールアドレスとパスワードの組み合わせで、他の通販サイトなどへのログインを試す。",
        case: "流出したIDとパスワードのリストを使った不正ログインで、通販サイトなどの被害が繰り返し起きています。",
        counter: "パスワードを使い回さない。二段階認証やパスキーを使う。",
        slots: &[slot("パスワード", &[Password]), slot("メールアドレス", &[Email])], illegal: true, filters: false,
    },
    ActionDef {
        key: "fake_follow", name: "なりすましフォロー申請", icon: "🎭", cat: "se", minutes: 2880, time_text: "2日（承認待ち）",
        desc: "知り合いを装った偽アカウントから鍵アカウントにフォロー申請し、承認されれば非公開の投稿を見る。",
        case: "同級生や知人の名前・写真を使った偽アカウントからのフォロー申請は、ストーカー行為やいじめの手口として知られています。",
        counter: "本人と確認できないアカウントの申請は承認しない。",
        slots: &[S_HANDLE], illegal: false, filters: false,
    },
    ActionDef {
        key: "friend_dm", name: "友人を装ってDM", icon: "💌", cat: "se", minutes: 1440, time_text: "1日（やりとり）",
        desc: "共通の友人や同じ趣味の人を装ってDMを送り、会話の中で連絡先や生活パターンを聞き出す。",
        case: "SNSで知り合った相手に住所や学校を聞き出され、待ち伏せされる被害は、子どもを中心に毎年報告されています。",
        counter: "DMの相手が本当に本人か確かめる。会話の中で連絡先・学校・予定を教えない。",
        slots: &[S_HANDLE], illegal: false, filters: false,
    },
    ActionDef {
        key: "pretext_call", name: "知人を装って電話（プリテキスティング）", icon: "☎️", cat: "se", minutes: 30, time_text: "30分",
        desc: "同級生・同窓会の幹事・学校関係者などを装って電話し、「書類を送りたい」などの口実で住所や携帯番号を聞き出す。",
        case: "同窓会名簿の作成や学校関係者を装って個人情報を聞き出す電話は、名簿業者や詐欺グループの手口として注意喚起されています。",
        counter: "電話で住所や連絡先を聞かれたら一度切り、正式な連絡先にかけ直して確認する。",
        slots: &[slot("電話番号", &[HomePhone, Mobile])], illegal: false, filters: false,
    },
    ActionDef {
        key: "apo_call", name: "調査員を装った電話（アポ電）", icon: "📞", cat: "se", minutes: 30, time_text: "30分",
        desc: "役所の調査や金融機関を装って自宅に電話し、家族構成・在宅時間・資産状況を聞き出す。",
        case: "2019年頃から、電話で資産や在宅状況を探ったうえで押し入る「アポ電強盗」が相次ぎました。",
        counter: "在宅時も留守番電話にする。知らない相手に家族構成や資産の話をしない。",
        slots: &[slot("自宅電話", &[HomePhone])], illegal: false, filters: false,
    },
    ActionDef {
        key: "workplace_call", name: "取引先を装って職場に電話", icon: "🏢", cat: "se", minutes: 30, time_text: "30分",
        desc: "取引先や知人を装って勤務先に電話し、所属部署や出勤日・勤務時間を聞き出す。",
        case: "職場への問い合わせを装った探りの電話は、ストーカーや詐欺の下調べに使われることがあります。",
        counter: "従業員の在席状況や個人情報を外部に伝えないルールを職場で徹底する。",
        slots: &[slot("勤務先", &[Workplace]), slot("氏名", &[Name])], illegal: false, filters: false,
    },
    ActionDef {
        key: "smishing", name: "宅配業者を装ったSMS（スミッシング）", icon: "📦", cat: "se", minutes: 1440, time_text: "1日（反応待ち）",
        desc: "「お荷物のお届けについて」などのSMSを送り、偽サイトで氏名・住所・カード情報を入力させる。",
        case: "宅配業者や通信会社を装うSMSによるフィッシングは、国内で大量に送られています。",
        counter: "SMSのリンクは開かず、公式アプリや公式サイトから確認する。",
        slots: &[slot("携帯電話", &[Mobile])], illegal: true, filters: false,
    },
    ActionDef {
        key: "phishing", name: "偽ログインページへ誘導（フィッシング）", icon: "🎣", cat: "se", minutes: 2880, time_text: "2日（反応待ち）",
        desc: "サービスからの通知を装ったメールで偽のログインページに誘導し、パスワードを入力させる。ログインできれば登録住所や購入履歴も見られる。",
        case: "金融機関や通販サイトを装うフィッシングの報告件数は、近年急増しています。",
        counter: "メール内のリンクからログインしない。二段階認証やパスキーを使う。",
        slots: &[slot("メールアドレス", &[Email])], illegal: true, filters: false,
    },
    ActionDef {
        key: "fake_campaign", name: "偽プレゼント企画", icon: "🎁", cat: "se", minutes: 4320, time_text: "3日（応募待ち）",
        desc: "SNSで「フォロー＆DMで当選」などの偽キャンペーンを行い、賞品の送り先として氏名・住所・電話番号を送らせる。",
        case: "企業や有名人のアカウントを装った偽プレゼント企画で個人情報を集める手口が、SNS上で繰り返し確認されています。",
        counter: "公式マークや公式サイトで主催者を確認する。DMで住所を送らない。",
        slots: &[S_HANDLE], illegal: false, filters: false,
    },
    ActionDef {
        key: "quiz_app", name: "心理テスト・診断を装った質問", icon: "🔮", cat: "se", minutes: 2880, time_text: "2日（回答待ち）",
        desc: "「あなたの性格診断」などで、誕生日・ペットの名前・出身校・母親の名前を答えさせる。これらは「秘密の質問」の答えになりやすい。",
        case: "SNSで流行する診断の質問が「秘密の質問」の答えと重なるとして、海外の警察や消費者団体が注意を呼びかけています。",
        counter: "診断系の質問に個人的な事実を答えない。秘密の質問には推測できない答えを登録する。",
        slots: &[S_HANDLE], illegal: false, filters: false,
    },
    ActionDef {
        key: "fake_job", name: "偽の求人・バイト募集", icon: "💼", cat: "se", minutes: 4320, time_text: "3日（応募待ち）",
        desc: "高収入の短期バイトなどを装って応募させ、履歴書や身分証の画像を送らせる。",
        case: "いわゆる「闇バイト」の募集では、応募時に送らせた身分証や家族の情報を使って脅し、犯罪に加担させる手口が問題になっています。",
        counter: "SNS経由の求人に身分証の画像を送らない。怪しいと思ったら家族や警察に相談する。",
        slots: &[S_HANDLE], illegal: true, filters: false,
    },
    ActionDef {
        key: "password_reset", name: "秘密の質問でパスワード再設定", icon: "🗝", cat: "se", minutes: 60, time_text: "1時間",
        desc: "メールアカウントのパスワード再設定画面で、集めた情報を「秘密の質問」の答えとして入力する。成功すればメールの中身から多くの情報が得られる。",
        case: "海外では、公開情報から推測された秘密の質問の答えによって政治家のメールアカウントが乗っ取られた事件が起きています。",
        counter: "秘密の質問には推測できない答えを設定し、二段階認証を有効にする。",
        slots: &[slot("メールアドレス", &[Email]), slot("答えに使う情報", &[Pet, School, BirthDate, Family])], illegal: true, filters: false,
    },
    ActionDef {
        key: "trashing", name: "ゴミ漁り（トラッシング）", icon: "🗑", cat: "physical", minutes: 1440, time_text: "1日（収集日を待つ）",
        desc: "住所がわかった相手のゴミ集積所から、郵便物・明細書・レシート・薬袋などを回収する。",
        case: "ゴミから個人情報や業務情報を集める「トラッシング」は、古典的なソーシャルエンジニアリングとして知られています。",
        counter: "宛名や明細はシュレッダーにかけるか、塗りつぶしてから捨てる。",
        slots: &[slot("住所（番地まで）", &[Address])], illegal: false, filters: false,
    },
    ActionDef {
        key: "juminhyo", name: "住民票の不正取得（職務上請求の悪用）", icon: "🏛", cat: "physical", minutes: 10080, time_text: "7日（請求・郵送）",
        desc: "依頼を受けた一部の士業が職務上請求書を不正に使い、住民票や戸籍を取得する。引っ越し先や家族構成まで判明する。",
        case: "探偵業者の依頼で、行政書士などが住民票や戸籍を大量に不正取得していた事件が過去に摘発されています。",
        counter: "多くの自治体にある「本人通知制度」に登録すると、第三者が住民票などを取得したときに通知を受けられる。",
        slots: &[slot("氏名", &[Name]), slot("住所の手がかり", &[Address, OldAddress, Town, City])], illegal: true, filters: false,
    },
];

#[derive(Serialize)]
pub struct SlotInfo {
    pub label: &'static str,
    pub fields: Vec<&'static str>,
    pub photo: u8,
}

#[derive(Serialize)]
pub struct ActionInfo {
    pub key: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub cat: &'static str,
    pub cat_label: &'static str,
    pub time_text: &'static str,
    pub desc: &'static str,
    pub case: &'static str,
    pub counter: &'static str,
    pub illegal: bool,
    pub filters: bool,
    pub slots: Vec<SlotInfo>,
}

pub fn catalog() -> Vec<ActionInfo> {
    ACTIONS
        .iter()
        .map(|a| ActionInfo {
            key: a.key,
            name: a.name,
            icon: a.icon,
            cat: a.cat,
            cat_label: CATS.iter().find(|c| c.0 == a.cat).map(|c| c.1).unwrap_or(""),
            time_text: a.time_text,
            desc: a.desc,
            case: a.case,
            counter: a.counter,
            illegal: a.illegal,
            filters: a.filters,
            slots: a.slots.iter().map(|s| SlotInfo { label: s.label, fields: s.fields.iter().map(|f| f.label()).collect(), photo: s.photo }).collect(),
        })
        .collect()
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Req {
    pub action: String,
    pub target: u32,
    pub fact: u32,
    pub fact2: u32,
    pub clue: u32,
    pub token: u32,
    pub page: u32,
    pub use_pref: bool,
    pub use_year: bool,
}

#[derive(Serialize, Default)]
pub struct Resp {
    pub ok: bool,
    pub message: String,
    pub detail: Vec<String>,
    pub minutes: i64,
    pub time_text: String,
    pub new_facts: Vec<FactView>,
    pub new_clues: Vec<ClueView>,
    pub feed: Option<FeedView>,
    pub accounts: Option<Vec<AccountView>>,
    pub hits: Option<HitsView>,
    pub new_target: Option<u32>,
    pub events: Vec<Event>,
    pub target: Option<u32>,
}

#[derive(Default)]
struct Out {
    ok: bool,
    message: String,
    detail: Vec<String>,
    cost: i64,
    facts: Vec<u32>,
    clues: Vec<u32>,
    feed: Option<FeedView>,
    accounts: Option<Vec<AccountView>>,
    hits: Option<HitsView>,
    new_target: Option<u32>,
}

impl Out {
    fn new(ok: bool, cost: i64, message: impl Into<String>) -> Out {
        Out { ok, cost, message: message.into(), ..Default::default() }
    }
}

impl Sim {
    pub fn run(&mut self, q: &Req) -> Resp {
        let t = q.target as usize;
        let (name, res) = match q.action.as_str() {
            "extract" => ("投稿から情報を記録", self.do_extract(t, q.token)),
            "extract_profile" => ("プロフィールを記録", self.do_extract_profile(t, q.token)),
            "link_account" => ("アカウントを紐付け", self.do_link_account(t, q.token)),
            "new_target_account" => ("アカウントを調査対象に追加", self.do_new_target_account(q.token)),
            "link_hit" => ("流出レコードを紐付け", self.do_link_hit(t, q.token)),
            "new_target_hit" => ("流出レコードの人物を調査対象に追加", self.do_new_target_hit(q.token)),
            key => match ACTIONS.iter().position(|a| a.key == key) {
                Some(ai) => (ACTIONS[ai].name, self.exec(ai, t, q)),
                None => return Resp { message: "不明な操作です".into(), ..Default::default() },
            },
        };
        match res {
            Ok(out) => self.finish(t, name, out),
            Err(e) => Resp { ok: false, message: e, ..Default::default() },
        }
    }

    fn finish(&mut self, t: usize, name: &str, out: Out) -> Resp {
        let events = self.spend(out.cost);
        let day = self.day();
        if let Some(tg) = self.targets.get_mut(t) {
            tg.spent += out.cost;
            for f in tg.facts.iter_mut().filter(|f| out.facts.contains(&f.id)) {
                f.day = day;
            }
            tg.log.push(LogEntry { day, action: name.to_string(), result: out.message.clone(), cost: out.cost, ok: out.ok });
        }
        Resp {
            ok: out.ok,
            message: out.message,
            detail: out.detail,
            minutes: out.cost,
            time_text: fmt_dur(out.cost),
            new_facts: out.facts.iter().filter_map(|&id| self.fact_view(id)).collect(),
            new_clues: out.clues.iter().filter_map(|&id| self.clue_view(id)).collect(),
            feed: out.feed,
            accounts: out.accounts,
            hits: out.hits,
            new_target: out.new_target,
            events,
            target: Some(t as u32),
        }
    }

    /// 入力された情報が手口の入力欄に合っているか確認する
    fn need(&self, t: usize, id: u32, s: &Slot) -> Result<FactInit, String> {
        let f = self.fact_of(t, id)?;
        if !s.fields.contains(&f.init.field) {
            return Err(format!("「{}」には {} が必要です", s.label, s.fields.iter().map(|f| f.label()).collect::<Vec<_>>().join("・")));
        }
        if f.init.subject as usize >= self.w.persons.len() {
            return Err("この情報は特定の人物と結び付いていないため使えません".into());
        }
        Ok(f.init.clone())
    }

    fn put(&mut self, t: usize, out: &mut Out, init: Option<FactInit>, source: &str, via: &str, parents: &[Parent], note: &str) {
        if let Some(init) = init {
            if let Some(id) = self.add_fact(t, init, source, via, parents.to_vec(), out.cost, note.to_string()) {
                out.facts.push(id);
            }
        }
    }

    fn exec(&mut self, ai: usize, t: usize, q: &Req) -> Result<Out, String> {
        if t >= self.targets.len() {
            return Err("調査対象が見つかりません".into());
        }
        match ACTIONS[ai].key {
            "sns_view" => self.a_sns_view(ai, t, q.fact, q.page),
            "sns_followers" => self.a_followers(ai, t, q.fact),
            "contact_sync" => self.a_contact_sync(ai, t, q.fact),
            "name_search" => self.a_name_search(ai, t, q.fact),
            "id_reuse" => self.a_id_reuse(ai, t, q.fact),
            "face_search" => self.a_face_search(ai, t, q.fact),
            "photo_reflect" => self.a_reflect(ai, t, q.clue),
            "photo_exif" => self.a_exif(ai, t, q.clue),
            "street_view" => self.a_street(ai, t, q.clue, q.fact),
            "deduce_birth" => self.a_deduce(ai, t, q.fact, q.fact2),
            "leak_search" => self.a_leak(ai, t, q.fact, q.use_pref, q.use_year),
            "list_broker" => self.a_broker(ai, t, q.fact, q.fact2),
            "credential_stuffing" => self.a_stuffing(ai, t, q.fact, q.fact2),
            "password_reset" => self.a_reset(ai, t, q.fact, q.fact2),
            "fake_follow" => self.a_fake_follow(ai, t, q.fact),
            "workplace_call" => self.a_workplace(ai, t, q.fact, q.fact2),
            "juminhyo" => self.a_juminhyo(ai, t, q.fact, q.fact2),
            _ => self.a_se(ai, t, q.fact),
        }
    }

    // ---- 公開情報 ----

    fn a_sns_view(&mut self, ai: usize, t: usize, fid: u32, page: u32) -> Result<Out, String> {
        let f = self.need(t, fid, &ACTIONS[ai].slots[0])?;
        let (pid, pf) = (f.subject, f.aux as u8);
        if self.w.p(pid).account(pf).is_none() {
            return Err("アカウントが見つかりません".into());
        }
        let account = self.account_view(pid, pf, fid, ai as u8, String::new());
        let profile_token = account.token;
        let locked = !account.accessible;
        let profile_info = !self.w.p(pid).account(pf).unwrap().bio.is_empty();
        let per = 10usize;
        let mut posts = Vec::new();
        let mut total = 0;
        if !locked {
            let now = self.day();
            let visible: Vec<usize> = {
                let feed = self.feed(pid, pf);
                (0..feed.posts.len()).filter(|&i| feed.posts[i].day <= now).collect()
            };
            total = visible.len();
            for &i in visible.iter().rev().skip(page as usize * per).take(per) {
                let (day, text, photo, info) = {
                    let p = &self.feeds[&(pid, pf)].posts[i];
                    (p.day, p.text.clone(), p.photo.as_ref().map(|x| x.desc.clone()), p.has_info())
                };
                let token = self.token(Ref::Post { pid, pf, idx: i as u32, via: fid });
                posts.push(PostView { token, date: date::fmt(day), text, photo, info, done: self.done.contains(&token) });
            }
        }
        let pages = total.div_ceil(per).max(1) as u32;
        let message = if locked {
            "鍵アカウントのため投稿は見られない（プロフィールのみ）".to_string()
        } else if total == 0 {
            "まだ投稿がない".to_string()
        } else {
            format!("{}件の投稿のうち {}〜{}件目を読んだ", total, page as usize * per + 1, (page as usize * per + posts.len()).min(total))
        };
        let mut out = Out::new(!locked, if locked { 5 } else { ACTIONS[ai].minutes }, message);
        out.feed = Some(FeedView {
            profile_token,
            profile_info,
            profile_done: self.done.contains(&profile_token),
            account,
            locked,
            posts,
            page,
            pages,
            total,
            fact: fid,
        });
        Ok(out)
    }

    fn do_extract(&mut self, t: usize, token: u32) -> Result<Out, String> {
        let Ref::Post { pid, pf, idx, via } = self.resolve(token)? else { return Err("無効な操作です".into()) };
        if t >= self.targets.len() {
            return Err("調査対象が見つかりません".into());
        }
        if self.done.contains(&token) {
            return Err("この投稿は記録済みです".into());
        }
        let (day, text, reveals, photo) = {
            let p = self.feeds.get(&(pid, pf)).and_then(|f| f.posts.get(idx as usize)).ok_or("投稿が見つかりません")?;
            (p.day, p.text.clone(), p.reveals.clone(), p.photo.as_ref().map(|x| (x.desc.clone(), x.features.clone(), x.exif)))
        };
        let src = format!("{}の投稿（{}）", PLATFORMS[pf as usize].0, date::fmt(day));
        let mut out = Out::new(true, 5, "");
        for (f, v) in reveals {
            let init = facts::value(&self.w, pid, f, v, day);
            self.put(t, &mut out, init, &src, "SNSの投稿を見る", &[Parent::Fact(via)], &text);
        }
        if let Some((caption, features, exif)) = photo {
            let c = Clue {
                id: 0,
                subject: pid,
                platform: pf,
                text: text.clone(),
                caption,
                day,
                features,
                exif,
                analyzed: false,
                exif_checked: false,
                parent: Some(via),
                cost: 5,
                total: 0,
            };
            if let Some(id) = self.add_clue(t, c) {
                out.clues.push(id);
            }
        }
        self.done.insert(token);
        out.message = if out.facts.is_empty() && out.clues.is_empty() {
            "新しい情報はなかった".into()
        } else {
            format!("情報{}件・写真{}枚を記録した", out.facts.len(), out.clues.len())
        };
        Ok(out)
    }

    fn do_extract_profile(&mut self, t: usize, token: u32) -> Result<Out, String> {
        let Ref::Account { pid, pf, via, .. } = self.resolve(token)? else { return Err("無効な操作です".into()) };
        if t >= self.targets.len() {
            return Err("調査対象が見つかりません".into());
        }
        if self.done.contains(&token) {
            return Err("このプロフィールは記録済みです".into());
        }
        let bio = self.w.p(pid).account(pf).map(|a| a.bio.clone()).unwrap_or_default();
        let src = format!("{}のプロフィール", PLATFORMS[pf as usize].0);
        let mut out = Out::new(true, 5, "");
        let now = self.day();
        for (f, v) in bio {
            let init = facts::value(&self.w, pid, f, v, now);
            self.put(t, &mut out, init, &src, "SNSのプロフィールを見る", &[Parent::Fact(via)], "");
        }
        self.done.insert(token);
        out.message = format!("プロフィールから{}件の情報を記録した", out.facts.len());
        Ok(out)
    }

    fn a_followers(&mut self, ai: usize, t: usize, fid: u32) -> Result<Out, String> {
        let f = self.need(t, fid, &ACTIONS[ai].slots[0])?;
        let (pid, pf) = (f.subject, f.aux as u8);
        if !self.accessible(pid, pf) {
            return Ok(Out::new(false, 5, "鍵アカウントのためフォロー一覧は見られない"));
        }
        let me = self.persona;
        let mut ids: Vec<u32> = self.w.p(pid).friends.iter().copied().filter(|&x| x != me && self.w.p(x).account(pf).is_some()).take(12).collect();
        for e in self.w.extended_friends(pid, pf) {
            if !ids.contains(&e) {
                ids.push(e);
            }
        }
        let accounts: Vec<AccountView> = ids.into_iter().map(|id| self.account_view(id, pf, fid, ai as u8, "フォロー／フォロワー".into())).collect();
        let mut out = Out::new(true, ACTIONS[ai].minutes, format!("{}件のアカウントがつながっていた", accounts.len()));
        out.accounts = Some(accounts);
        Ok(out)
    }

    fn a_contact_sync(&mut self, ai: usize, t: usize, fid: u32) -> Result<Out, String> {
        let f = self.need(t, fid, &ACTIONS[ai].slots[0])?;
        let key = f.key.clone();
        let mut res = Vec::new();
        for p in &self.w.persons {
            if p.kind == Kind::Idol {
                continue;
            }
            let hit = match f.field {
                Mobile => p.mobile.as_deref() == Some(key.as_str()),
                _ => p.emails.contains(&key) || p.work_email.as_deref() == Some(key.as_str()) || p.school_email.as_deref() == Some(key.as_str()),
            };
            if !hit {
                continue;
            }
            for a in &p.accounts {
                if (f.field == Mobile && a.by_phone) || (f.field == Email && a.by_email) {
                    res.push((p.id, a.platform));
                }
            }
        }
        let note = if f.field == Mobile { "電話番号で見つかった" } else { "メールアドレスで見つかった" };
        let accounts: Vec<AccountView> = res.into_iter().map(|(id, pf)| self.account_view(id, pf, fid, ai as u8, note.into())).collect();
        let msg = if accounts.is_empty() {
            "この連絡先で検索できるアカウントはなかった（検索を許可していないか、SNSを使っていない）".to_string()
        } else {
            format!("{}件のアカウントが見つかった", accounts.len())
        };
        let mut out = Out::new(!accounts.is_empty(), ACTIONS[ai].minutes, msg);
        out.accounts = Some(accounts);
        Ok(out)
    }

    fn a_name_search(&mut self, ai: usize, t: usize, fid: u32) -> Result<Out, String> {
        let f = self.need(t, fid, &ACTIONS[ai].slots[0])?;
        let name = f.value.clone();
        let mut ids: Vec<u32> = self.w.persons.iter().filter(|p| p.account(2).is_some() && p.full_name() == name).map(|p| p.id).collect();
        if let Some((sk, gk)) = name.split_once(' ') {
            if let (Some(s), Some((_, _, g))) = (SURNAMES.iter().position(|x| x.0 == sk), find_given(gk)) {
                let freq = SURNAMES[s].3 * 1e4 / JAPAN_POP;
                let mut n = ((freq * 300.0).round() as usize).min(5);
                if hash01(&[self.w.seed, 0x4EA1, hash_str(&name)]) < 0.5 {
                    n += 1;
                }
                for x in self.w.namesakes(mix(&[0x4EA1, hash_str(&name)]), s, g, n, Some(2), None) {
                    if !ids.contains(&x) {
                        ids.push(x);
                    }
                }
            }
        }
        let seed = self.w.seed;
        ids.sort_by_key(|&id| mix(&[seed, id as u64]));
        let accounts: Vec<AccountView> = ids.into_iter().map(|id| self.account_view(id, 2, fid, ai as u8, String::new())).collect();
        let msg = if accounts.is_empty() {
            format!("「{}」のアカウントは見つからなかった", name)
        } else {
            format!("「{}」で{}件のアカウントが見つかった。居住地や学歴を見て本人か見極める必要がある", name, accounts.len())
        };
        let mut out = Out::new(!accounts.is_empty(), ACTIONS[ai].minutes, msg);
        out.accounts = Some(accounts);
        Ok(out)
    }

    fn a_id_reuse(&mut self, ai: usize, t: usize, fid: u32) -> Result<Out, String> {
        let f = self.need(t, fid, &ACTIONS[ai].slots[0])?;
        let (owner, pf0, key) = (f.subject, f.aux as u8, f.key.clone());
        let mut res = Vec::new();
        for p in &self.w.persons {
            for a in &p.accounts {
                if a.handle == key && !(p.id == owner && a.platform == pf0) {
                    res.push((p.id, a.platform));
                }
            }
        }
        let accounts: Vec<AccountView> = res.into_iter().map(|(id, pf)| self.account_view(id, pf, fid, ai as u8, format!("同じID「{}」", key))).collect();
        let msg = if accounts.is_empty() { "同じIDのアカウントは見つからなかった".to_string() } else { format!("同じIDのアカウントが{}件見つかった", accounts.len()) };
        let mut out = Out::new(!accounts.is_empty(), ACTIONS[ai].minutes, msg);
        out.accounts = Some(accounts);
        Ok(out)
    }

    fn a_face_search(&mut self, ai: usize, t: usize, fid: u32) -> Result<Out, String> {
        let f = self.need(t, fid, &ACTIONS[ai].slots[0])?;
        let s = f.subject;
        let seed = self.w.seed;
        let mut res: Vec<(u32, u8, u32)> = self
            .w
            .p(s)
            .accounts
            .iter()
            .filter(|a| a.platform != 2 && a.public && a.selfies)
            .map(|a| (s, a.platform, 88 + (hash01(&[seed, 0xFA, s as u64, a.platform as u64]) * 10.0) as u32))
            .collect();
        for l in self.w.lookalikes(s) {
            res.push((l, 1, 70 + (hash01(&[seed, 0xFA, s as u64, l as u64]) * 15.0) as u32));
        }
        res.sort_by_key(|x| std::cmp::Reverse(x.2));
        let accounts: Vec<AccountView> = res.into_iter().map(|(id, pf, sim)| self.account_view(id, pf, fid, ai as u8, format!("顔の類似度 {}%", sim))).collect();
        let msg = if accounts.is_empty() { "一致する顔写真は見つからなかった".to_string() } else { format!("{}件の候補が見つかった。似ている別人の可能性もある", accounts.len()) };
        let mut out = Out::new(!accounts.is_empty(), ACTIONS[ai].minutes, msg);
        out.accounts = Some(accounts);
        Ok(out)
    }

    fn do_link_account(&mut self, t: usize, token: u32) -> Result<Out, String> {
        let Ref::Account { pid, pf, via, how } = self.resolve(token)? else { return Err("無効な操作です".into()) };
        if t >= self.targets.len() {
            return Err("調査対象が見つかりません".into());
        }
        let how = ACTIONS[how as usize].name;
        let src = format!("{}（{}）", how, PLATFORMS[pf as usize].0);
        let now = self.day();
        let mut out = Out::new(true, 2, "");
        let h = facts::value(&self.w, pid, Handle, pf, now);
        self.put(t, &mut out, h, &src, how, &[Parent::Fact(via)], "");
        let n = facts::value(&self.w, pid, if pf == 2 { Name } else { Alias }, 0, now);
        self.put(t, &mut out, n, &src, how, &[Parent::Fact(via)], "");
        out.message = if out.facts.is_empty() { "すでに紐付け済み".into() } else { "アカウントをこの人物のものとして記録した".into() };
        Ok(out)
    }

    fn do_new_target_account(&mut self, token: u32) -> Result<Out, String> {
        let Ref::Account { pid, pf, via, how } = self.resolve(token)? else { return Err("無効な操作です".into()) };
        let display = self.w.p(pid).account(pf).map(|a| a.display.clone()).unwrap_or_default();
        let how = ACTIONS[how as usize].name;
        let nt = self.new_target(pid, display, &format!("「{}」で見つけた人物", how));
        let src = format!("{}（{}）", how, PLATFORMS[pf as usize].0);
        let now = self.day();
        let mut out = Out::new(true, 0, "調査対象に追加した");
        for (f, v) in [(Handle, pf), (if pf == 2 { Name } else { Alias }, 0)] {
            let init = facts::value(&self.w, pid, f, v, now);
            self.put(nt, &mut out, init, &src, how, &[Parent::Fact(via)], "");
        }
        out.facts.clear();
        out.new_target = Some(nt as u32);
        Ok(out)
    }

    // ---- 写真・推理 ----

    fn a_reflect(&mut self, ai: usize, t: usize, cid: u32) -> Result<Out, String> {
        let c = self.clue_of(t, cid)?;
        if c.analyzed {
            return Err("この写真は解析済みです".into());
        }
        let (pid, pf, features) = (c.subject, c.platform, c.features.clone());
        let now = self.day();
        let mut out = Out::new(true, ACTIONS[ai].minutes, "");
        let src = format!("{}の写真の写り込み", PLATFORMS[pf as usize].0);
        for feat in &features {
            let w = &self.w;
            let a = w.addr(pid);
            let (msg, init) = match feat {
                Feature::Face => ("顔がはっきり写っている → 顔画像として保存".to_string(), Some(FactInit::new(Face, format!("顔写真（{}の投稿）", PLATFORMS[pf as usize].0), pid))),
                Feature::EyeReflection => (format!("瞳を拡大すると駅の看板と街並みが映り込んでいる → {}周辺と推定", w.station_of(pid).name), facts::value(w, pid, Station, 0, now)),
                Feature::PolePlate => (format!("背景の電柱の住所表示板に「{}{}丁目」と書かれている", w.town_of(pid).name, a.chome), facts::value(w, pid, Town, 1, now)),
                Feature::DeliverySlip => (
                    format!("宅配伝票の一部が読める：「〒{}」「{} 様」", w.town_of(pid).postal, w.p(pid).sur_kanji()),
                    facts::value(w, pid, Postal, 0, now),
                ),
                Feature::Uniform => (format!("制服と校章の形から「{}」の生徒とわかる", w.p(pid).school.clone().unwrap_or_default()), facts::value(w, pid, School, 0, now)),
                Feature::IdCard => (format!("社員証のロゴから「{}」の社員とわかる", w.p(pid).workplace.clone().unwrap_or_default()), facts::value(w, pid, Workplace, 0, now)),
                Feature::Landmark => (format!("遠くに見える建物と地形の特徴から{}{}付近と推定", CITIES[a.city].1, w.town_of(pid).name), facts::value(w, pid, Town, 0, now)),
                Feature::CarPlate => (format!("ナンバープレートの地名表示から{}内に住んでいると推定", PREFS[a.pref].0), facts::value(w, pid, Pref, 0, now)),
                Feature::HouseView => ("家の外観と周囲の景色が写っている → 場所の見当がつけば「ストリートビューで照合」に使える".to_string(), None),
            };
            out.detail.push(format!("🔍 {}：{}", feat.label(), msg));
            self.put(t, &mut out, init, &src, &format!("写り込みを調べる（{}）", feat.label()), &[Parent::Clue(cid)], &msg);
        }
        if features.is_empty() {
            out.detail.push("特に手がかりになるものは写っていなかった".into());
        }
        if let Some(c) = self.clue_mut(cid) {
            c.analyzed = true;
        }
        out.message = format!("写真を拡大して調べた（手がかり{}件）", features.len());
        out.ok = !features.is_empty();
        Ok(out)
    }

    fn a_exif(&mut self, ai: usize, t: usize, cid: u32) -> Result<Out, String> {
        let c = self.clue_of(t, cid)?;
        if c.exif_checked {
            return Err("この写真の位置情報は確認済みです".into());
        }
        let (pid, exif) = (c.subject, c.exif);
        if let Some(c) = self.clue_mut(cid) {
            c.exif_checked = true;
        }
        if !exif {
            return Ok(Out::new(false, ACTIONS[ai].minutes, "位置情報は削除されていた"));
        }
        let mut out = Out::new(true, ACTIONS[ai].minutes, "写真に撮影位置の情報が残っていた！ 住所が判明");
        let init = facts::value(&self.w, pid, Address, 0, self.day());
        let a = self.w.addr(pid);
        let note = format!("撮影位置 北緯{:.5} 東経{:.5}", a.lat, a.lon);
        out.detail.push(note.clone());
        self.put(t, &mut out, init, "写真の位置情報（Exif）", ACTIONS[ai].name, &[Parent::Clue(cid)], &note);
        Ok(out)
    }

    fn a_street(&mut self, ai: usize, t: usize, cid: u32, fid: u32) -> Result<Out, String> {
        let c = self.clue_of(t, cid)?;
        if !(c.analyzed && c.features.contains(&Feature::HouseView)) {
            return Err("家の外観や景色が写った、解析済みの写真が必要です".into());
        }
        let csub = c.subject;
        let f = self.need(t, fid, &ACTIONS[ai].slots[1])?;
        let (cost, p, area) = match f.field {
            Town if f.aux > 0 => (60, 0.92, "丁目内"),
            Town => (150, 0.7, "町内"),
            _ => (300, 0.4, "駅周辺"),
        };
        let ok = f.subject == csub && hash01(&[self.w.seed, 0x57EE7, cid as u64, fid as u64]) < p;
        if ok {
            let mut out = Out::new(true, cost, format!("{}の街並みを片端から見て、写真と同じ外観の家を見つけた。住所が判明", area));
            let init = facts::value(&self.w, csub, Address, 0, self.day());
            self.put(t, &mut out, init, "地図サービスの街並み画像との照合", ACTIONS[ai].name, &[Parent::Clue(cid), Parent::Fact(fid)], "");
            Ok(out)
        } else if f.subject != csub {
            Ok(Out::new(false, cost, format!("{}をくまなく見たが、写真と一致する場所は見つからなかった（場所の推定が間違っているかもしれない）", f.value)))
        } else {
            Ok(Out::new(false, cost, format!("{}を探したが、範囲が広すぎて特定できなかった。もっと範囲を絞れる手がかりが必要", area)))
        }
    }

    fn a_deduce(&mut self, ai: usize, t: usize, f1: u32, f2: u32) -> Result<Out, String> {
        let md = self.need(t, f1, &ACTIONS[ai].slots[0])?;
        let ag = self.need(t, f2, &ACTIONS[ai].slots[1])?;
        let (m, d) = ((md.aux / 100) as u32, (md.aux % 100) as u32);
        let by = if ag.aux2 == -1 {
            let y = ag.aux as i32;
            if (m, d) >= (4, 2) {
                y
            } else {
                y + 1
            }
        } else {
            let (y, mm, dd) = date::civil_from_days(ag.aux2 as i32);
            let age = ag.aux as i32;
            if (m, d) <= (mm, dd) {
                y - age
            } else {
                y - age - 1
            }
        };
        let day = date::days_from_civil(by, m, d);
        let subject = if md.subject == ag.subject { md.subject } else { u32::MAX };
        let mut init = FactInit::new(BirthDate, date::fmt(day), subject);
        init.aux = day as i64;
        let mut out = Out::new(true, ACTIONS[ai].minutes, format!("{}と「{}」から、生年月日は{}と割り出した", md.value, ag.value, date::fmt(day)));
        self.put(t, &mut out, Some(init), "断片の組み合わせ", ACTIONS[ai].name, &[Parent::Fact(f1), Parent::Fact(f2)], "");
        if out.facts.is_empty() {
            out.message = "すでにわかっている生年月日と同じだった".into();
        }
        Ok(out)
    }

    // ---- 流出データ ----

    fn person_has_key(&self, id: u32, field: Field, key: &str) -> bool {
        let p = self.w.p(id);
        match field {
            Name => p.full_name() == key,
            Email => p.emails.iter().any(|e| e == key) || p.school_email.as_deref() == Some(key),
            Mobile => p.mobile.as_deref() == Some(key),
            HomePhone => self.w.households[p.household].phone.as_deref() == Some(key),
            Address | OldAddress => self.w.address_text(self.w.addr(id)) == key,
            Handle => p.base_handle == key || p.accounts.iter().any(|a| a.handle == key),
            _ => false,
        }
    }

    fn a_leak(&mut self, ai: usize, t: usize, fid: u32, use_pref: bool, use_year: bool) -> Result<Out, String> {
        let f = self.need(t, fid, &ACTIONS[ai].slots[0])?;
        let key_field = f.field;
        let rec_field = if key_field == OldAddress { Address } else { key_field };
        let key = if key_field == Name { f.value.clone() } else { f.key.clone() };
        let mut filters = Vec::new();
        let tg = &self.targets[t];
        let pref_filter = if use_pref {
            tg.facts
                .iter()
                .filter(|x| x.init.pref.is_some() && x.init.field != OldAddress)
                .min_by(|a, b| a.init.geo.map(|g| g.r).unwrap_or(1e9).partial_cmp(&b.init.geo.map(|g| g.r).unwrap_or(1e9)).unwrap())
                .and_then(|x| x.init.pref)
        } else {
            None
        };
        if let Some(p) = pref_filter {
            filters.push(format!("居住地: {}", PREFS[p].0));
        }
        let year_filter = if use_year {
            tg.facts.iter().find(|x| x.init.field == BirthDate).map(|x| (date::year(x.init.aux as i32), date::year(x.init.aux as i32))).or_else(|| {
                tg.facts.iter().find(|x| x.init.field == Age).map(|x| {
                    if x.init.aux2 == -1 {
                        (x.init.aux as i32, x.init.aux as i32 + 1)
                    } else {
                        let y = date::year(x.init.aux2 as i32) - x.init.aux as i32;
                        (y - 1, y)
                    }
                })
            })
        } else {
            None
        };
        if let Some((a, b)) = year_filter {
            filters.push(if a == b { format!("生まれ年: {}年", a) } else { format!("生まれ年: {}〜{}年", a, b) });
        }
        if (use_pref && pref_filter.is_none()) || (use_year && year_filter.is_none()) {
            filters.push("（絞り込みに使える情報が足りない条件は無視）".into());
        }

        let circ: Vec<usize> = (0..self.incidents.len())
            .filter(|&i| {
                let inc = &self.incidents[i];
                inc.circ.is_some_and(|c| c <= self.day()) && inc.fields.contains(&rec_field)
            })
            .collect();
        let mut est_map: std::collections::HashMap<u32, f64> = Default::default();
        if key_field == Name {
            if let Some((sk, gk)) = key.split_once(' ') {
                if let (Some(s), Some((_, _, g))) = (SURNAMES.iter().position(|x| x.0 == sk), find_given(gk)) {
                    let freq = SURNAMES[s].3 * 1e4 / JAPAN_POP;
                    let mut ests: Vec<(u32, f64)> = circ.iter().map(|&i| (self.incidents[i].id, self.incidents[i].count as f64 * freq * 0.0022)).collect();
                    ests.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                    for (k, &(iid, est)) in ests.iter().enumerate() {
                        est_map.insert(iid, est);
                        if k < 8 && est >= 0.3 {
                            let frac = est - est.floor();
                            let n = (est.floor() as usize + usize::from(hash01(&[self.w.seed, 0x4A5F, iid as u64]) < frac)).min(3);
                            self.w.namesakes(mix(&[0x4A5E, iid as u64, hash_str(&key)]), s, g, n, None, Some(iid));
                        }
                    }
                }
            }
        }
        let cands: Vec<u32> = (0..self.w.persons.len() as u32).filter(|&id| self.person_has_key(id, key_field, &key)).collect();
        let pshare = pref_filter.map(|p| PREFS[p].3 / PREFS.iter().map(|x| x.3).sum::<f64>()).unwrap_or(1.0);
        let yshare = if year_filter.is_some() { 0.12 } else { 1.0 };
        let mut found: Vec<(usize, Vec<(u32, Vec<FactInit>)>, Option<f64>)> = Vec::new();
        for &i in circ.iter().rev() {
            let inc = &self.incidents[i];
            let mut hits = Vec::new();
            for &c in &cands {
                if !incident::leaked(&self.w, inc, c) {
                    continue;
                }
                let rec = incident::record(&self.w, inc, c);
                if !rec.iter().any(|r| r.field == rec_field && if key_field == Name { r.value == key } else { r.key == key }) {
                    continue;
                }
                if let (Some(pf), Some(rp)) = (pref_filter, rec.iter().find_map(|r| r.pref)) {
                    if pf != rp {
                        continue;
                    }
                }
                if let (Some((y0, y1)), Some(b)) = (year_filter, rec.iter().find(|r| r.field == BirthDate)) {
                    let y = date::year(b.aux as i32);
                    if y < y0 || y > y1 {
                        continue;
                    }
                }
                hits.push((c, rec));
            }
            let est = est_map.get(&inc.id).map(|e| e * pshare * yshare);
            if hits.is_empty() && est.is_none_or(|e| e < 1.0) {
                continue;
            }
            found.push((i, hits, est));
            if found.len() >= 30 {
                break;
            }
        }
        let mut groups = Vec::new();
        let mut total = 0;
        for (i, hits, est) in found {
            let iid = self.incidents[i].id;
            let mut hv = Vec::new();
            for (pid, rec) in hits {
                let token = self.token(Ref::Hit { inc: iid, pid, via: fid });
                hv.push(HitView { token, done: self.done.contains(&token), fields: rec.into_iter().map(|r| KV { label: r.field.label().into(), value: r.value }).collect() });
            }
            total += hv.len();
            let shown = hv.len() as f64;
            groups.push(HitGroup {
                incident: brief(&self.incidents[i]),
                namesakes: est.filter(|&e| e >= 1.0).map(|e| {
                    format!("このデータには同姓同名が推定{}件含まれる{}", incident::group(e.round() as u64), if shown < e.round() { "（一部のみ表示）" } else { "" })
                }),
                hits: hv,
            });
        }
        let msg = if total == 0 {
            format!("流通中の{}件のデータを検索したが、該当はなかった", circ.len())
        } else {
            format!("流通中の{}件のデータから{}件のレコードが見つかった", circ.len(), total)
        };
        let mut out = Out::new(total > 0, ACTIONS[ai].minutes, msg);
        out.hits = Some(HitsView {
            key: format!("{}: {}", key_field.label(), f.value),
            filters,
            groups,
            total,
            searched: circ.len(),
            note: if key_field == Name { "氏名だけの検索では同姓同名の別人が大量にヒットする。居住地や生まれ年で絞り込むと確度が上がる。".into() } else { String::new() },
        });
        Ok(out)
    }

    fn do_link_hit(&mut self, t: usize, token: u32) -> Result<Out, String> {
        let Ref::Hit { inc, pid, via } = self.resolve(token)? else { return Err("無効な操作です".into()) };
        if t >= self.targets.len() {
            return Err("調査対象が見つかりません".into());
        }
        if self.done.contains(&token) {
            return Err("このレコードは紐付け済みです".into());
        }
        let i = &self.incidents[inc as usize];
        let src = format!("流出データ: {}（{}公表）", i.company, date::fmt(i.announce));
        let rec = incident::record(&self.w, i, pid);
        let mut out = Out::new(true, 2, "");
        for r in rec {
            self.put(t, &mut out, Some(r), &src, "流出データで名寄せ", &[Parent::Fact(via)], "");
        }
        self.done.insert(token);
        out.message = format!("流出レコードを同一人物として紐付けた（新しい情報{}件）", out.facts.len());
        Ok(out)
    }

    fn do_new_target_hit(&mut self, token: u32) -> Result<Out, String> {
        let Ref::Hit { inc, pid, via } = self.resolve(token)? else { return Err("無効な操作です".into()) };
        if self.done.contains(&token) {
            return Err("このレコードは登録済みです".into());
        }
        let i = &self.incidents[inc as usize];
        let src = format!("流出データ: {}（{}公表）", i.company, date::fmt(i.announce));
        let rec = incident::record(&self.w, i, pid);
        let label = rec
            .iter()
            .find(|r| r.field == Name)
            .or_else(|| rec.iter().find(|r| r.field == Alias))
            .or_else(|| rec.iter().find(|r| r.field == Handle))
            .map(|r| r.value.clone())
            .unwrap_or_else(|| "（名前不明）".into());
        let nt = self.new_target(pid, label, "流出データで見つけた人物");
        let mut out = Out::new(true, 0, "別人として調査対象に追加した");
        for r in rec {
            self.put(nt, &mut out, Some(r), &src, "流出データで名寄せ", &[Parent::Fact(via)], "");
        }
        out.facts.clear();
        self.done.insert(token);
        out.new_target = Some(nt as u32);
        Ok(out)
    }

    fn a_broker(&mut self, ai: usize, t: usize, f1: u32, f2: u32) -> Result<Out, String> {
        let n = self.need(t, f1, &ACTIONS[ai].slots[0])?;
        let s = self.need(t, f2, &ACTIONS[ai].slots[1])?;
        let found = self
            .w
            .persons
            .iter()
            .find(|p| p.full_name() == n.value && (p.school.as_deref() == Some(s.key.as_str()) || p.past_schools.contains(&s.key)))
            .map(|p| (p.id, p.hometown.is_some()));
        let cost = ACTIONS[ai].minutes;
        let Some((pid, moved)) = found else {
            return Ok(Out::new(false, cost, "条件に合う名簿が見つからなかった"));
        };
        let mut out = Out::new(true, cost, format!("名簿業者から「{}」の卒業生名簿を購入した（1件あたり数千円）", s.key));
        let now = self.day();
        let src = "名簿業者の卒業生名簿";
        let parents = [Parent::Fact(f1), Parent::Fact(f2)];
        let (af, pv) = if moved { (OldAddress, 1) } else { (Address, 0) };
        let a = facts::value(&self.w, pid, af, 0, now);
        self.put(t, &mut out, a, src, ACTIONS[ai].name, &parents, "");
        let p = facts::value(&self.w, pid, HomePhone, pv, now);
        self.put(t, &mut out, p, src, ACTIONS[ai].name, &parents, "");
        if hash01(&[self.w.seed, 0xB20, pid as u64]) < 0.4 {
            let fam = facts::value(&self.w, pid, Family, 0, now);
            self.put(t, &mut out, fam, src, ACTIONS[ai].name, &parents, "");
        }
        Ok(out)
    }

    fn se_results(&mut self, t: usize, out: &mut Out, pid: u32, ai: usize, list: &[(Field, u8, f64)], parents: &[Parent]) {
        let n = self.attempt_no(pid, ai);
        let mut r = rng(&[self.w.seed, 0x5E5, pid as u64, ai as u64, n as u64]);
        let now = self.day();
        let src = ACTIONS[ai].name;
        for &(f, v, p) in list {
            if !r.chance(p) {
                continue;
            }
            let init = match (f, v) {
                (Family, 9) => self.w.mother_name(pid).map(|m| FactInit::new(Family, format!("母: {}", m), pid)),
                (Face, 9) => Some(FactInit::new(Face, "証明写真（履歴書に添付）".into(), pid)),
                _ => facts::value(&self.w, pid, f, v, now),
            };
            self.put(t, out, init, src, src, parents, "");
        }
        if out.facts.is_empty() {
            out.detail.push("ただし、目新しい情報は得られなかった".into());
        }
    }

    fn a_stuffing(&mut self, ai: usize, t: usize, f1: u32, f2: u32) -> Result<Out, String> {
        let pw = self.need(t, f1, &ACTIONS[ai].slots[0])?;
        let em = self.need(t, f2, &ACTIONS[ai].slots[1])?;
        let pid = em.subject;
        let p = self.w.p(pid);
        let cost = ACTIONS[ai].minutes;
        if pw.aux == 0 {
            return Ok(Out::new(false, 10, "解読できていないパスワードは使えない"));
        }
        if pw.aux != hash_str(&p.password) as i64 || !p.pass_reuse {
            return Ok(Out::new(false, cost, "いくつもの通販サイトで試したがログインできなかった（パスワードが使い回されていない）"));
        }
        let mut out = Out::new(true, cost, "同じパスワードで通販サイトにログインできてしまった。登録情報と購入履歴が丸見えになった");
        let list = [(Name, 0, 0.9), (Address, 0, 0.9), (History, 0, 0.9), (Card, 0, 0.6), (Mobile, 0, 0.7), (BirthDate, 0, 0.4)];
        self.se_results(t, &mut out, pid, ai, &list, &[Parent::Fact(f1), Parent::Fact(f2)]);
        Ok(out)
    }

    // ---- 対人 ----

    fn alert_of(&self, pid: u32) -> u32 {
        self.alert.get(&pid).copied().unwrap_or(0)
    }

    fn a_reset(&mut self, ai: usize, t: usize, f1: u32, f2: u32) -> Result<Out, String> {
        let em = self.need(t, f1, &ACTIONS[ai].slots[0])?;
        let ans = self.need(t, f2, &ACTIONS[ai].slots[1])?;
        let pid = em.subject;
        let cost = ACTIONS[ai].minutes;
        let p = self.w.p(pid);
        if !p.emails.contains(&em.key) {
            return Ok(Out::new(false, 10, "個人のメールアドレスではないため、この手口は使えなかった（会社や学校のアカウントは管理者が再設定する）"));
        }
        let q = match p.secq {
            Pet => "初めて飼ったペットの名前は？",
            School => "出身小学校の名前は？",
            BirthDate => "あなたの生年月日は？",
            _ => "母親の名前は？",
        };
        if self.alert_of(pid) >= 3 {
            return Ok(Out::new(false, cost, "試行回数が多すぎて、アカウントがロックされた"));
        }
        let correct = ans.subject == pid
            && match p.secq {
                Pet => ans.field == Pet,
                School => ans.field == School && p.past_schools.first() == Some(&ans.key),
                BirthDate => ans.field == BirthDate && ans.aux == p.birth as i64,
                _ => ans.field == Family && self.w.mother_name(pid).is_some_and(|m| ans.value.contains(&m)),
            };
        self.se_roll(pid, ai, if correct { 1.0 } else { 0.0 });
        let detail = format!("再設定画面に表示された秘密の質問：「{}」", q);
        if !correct {
            let mut out = Out::new(false, cost, "秘密の質問の答えが一致しなかった（相手にログイン試行の通知が届いたかもしれない）");
            out.detail.push(detail);
            return Ok(out);
        }
        let mut out = Out::new(true, cost, "秘密の質問を突破してメールアカウントを乗っ取った。通販の注文確認メールなどから大量の情報が得られた");
        out.detail.push(detail);
        let list = [(Password, 2, 1.0), (Name, 0, 0.9), (Address, 0, 0.8), (History, 0, 0.8), (Workplace, 0, 0.5), (Card, 0, 0.4), (Family, 0, 0.4), (Mobile, 0, 0.6)];
        self.se_results(t, &mut out, pid, ai, &list, &[Parent::Fact(f1), Parent::Fact(f2)]);
        Ok(out)
    }

    fn a_fake_follow(&mut self, ai: usize, t: usize, fid: u32) -> Result<Out, String> {
        let f = self.need(t, fid, &ACTIONS[ai].slots[0])?;
        let (pid, pf) = (f.subject, f.aux as u8);
        if self.accessible(pid, pf) {
            return Err("このアカウントはすでに投稿を見られます（公開または相互フォロー）".into());
        }
        let p = self.w.p(pid);
        let base = 0.25 + 0.5 * (1.0 - p.caution) + if p.age_at(self.day()) <= 19 { 0.1 } else { 0.0 };
        let cost = ACTIONS[ai].minutes;
        if self.se_roll(pid, ai, base) {
            self.fake_follows.insert((pid, pf));
            Ok(Out::new(true, cost, "フォロー申請が承認された。鍵アカウントの投稿が見られるようになった"))
        } else {
            Ok(Out::new(false, cost, "申請は承認されなかった。相手の警戒心が高まった"))
        }
    }

    fn a_workplace(&mut self, ai: usize, t: usize, f1: u32, f2: u32) -> Result<Out, String> {
        let wk = self.need(t, f1, &ACTIONS[ai].slots[0])?;
        let nm = self.need(t, f2, &ACTIONS[ai].slots[1])?;
        let pid = wk.subject;
        let cost = ACTIONS[ai].minutes;
        if nm.subject != pid || self.w.p(pid).workplace.is_none() {
            return Ok(Out::new(false, cost, "「そのような者はおりません」と言われた"));
        }
        let base = 0.45 + 0.35 * (1.0 - self.w.p(pid).caution);
        if !self.se_roll(pid, ai, base) {
            return Ok(Out::new(false, cost, "「個人のことはお答えできません」と断られた。本人に不審な電話があったと伝わったかもしれない"));
        }
        let mut out = Out::new(true, cost, "取引先を名乗ると、所属部署や出社予定をあっさり教えてくれた");
        self.se_results(t, &mut out, pid, ai, &[(Workplace, 1, 1.0), (Routine, 0, 0.8)], &[Parent::Fact(f1), Parent::Fact(f2)]);
        Ok(out)
    }

    fn a_juminhyo(&mut self, ai: usize, t: usize, f1: u32, f2: u32) -> Result<Out, String> {
        let nm = self.need(t, f1, &ACTIONS[ai].slots[0])?;
        let res = self.need(t, f2, &ACTIONS[ai].slots[1])?;
        let pid = nm.subject;
        let cost = ACTIONS[ai].minutes;
        if res.subject != pid {
            return Ok(Out::new(false, cost, "該当する住民票が見つからなかった（氏名と住所が一致しない）"));
        }
        if self.w.p(pid).caution > 0.85 {
            self.raise_alert(pid, 2);
            return Ok(Out::new(false, cost, "本人通知制度により、第三者が住民票を取得したことが本人に通知された。相手は強く警戒している"));
        }
        if !self.se_roll(pid, ai, 0.75) {
            return Ok(Out::new(false, cost, "役所の窓口で請求の不備を指摘され、取得できなかった"));
        }
        let mut out = Out::new(true, cost, "住民票と戸籍を取得した。現住所と家族構成が判明");
        let list = [(Name, 0, 1.0), (Kana, 0, 0.8), (Address, 0, 1.0), (Family, 0, 1.0), (BirthDate, 0, 1.0), (Field::Gender, 0, 1.0)];
        self.se_results(t, &mut out, pid, ai, &list, &[Parent::Fact(f1), Parent::Fact(f2)]);
        Ok(out)
    }

    /// 入力が1つで、相手の警戒心などで成否が決まる手口
    fn a_se(&mut self, ai: usize, t: usize, fid: u32) -> Result<Out, String> {
        let a = &ACTIONS[ai];
        let f = self.need(t, fid, &a.slots[0])?;
        let pid = f.subject;
        let p = self.w.p(pid);
        let (age, c, o) = (p.age_at(self.day()), p.caution, p.overshare);
        type Res = &'static [(Field, u8, f64)];
        let (base, list, ok_msg, ng_msg): (f64, Res, &str, &str) = match a.key {
            "friend_dm" => (
                0.2 + 0.5 * o * (1.0 - c),
                &[(Mobile, 0, 0.6), (Station, 0, 0.5), (School, 0, 0.4), (Workplace, 0, 0.4), (Routine, 0, 0.5), (BirthMd, 0, 0.3)],
                "共通の友人を装ったDMで会話が弾み、いろいろ聞き出せた",
                "返信はなかった。不審に思われたかもしれない",
            ),
            "pretext_call" => (
                0.25 + 0.45 * (1.0 - c) + if age >= 65 { 0.15 } else { 0.0 },
                &[(Address, 0, 0.8), (Mobile, 0, 0.4), (Family, 0, 0.5), (Routine, 0, 0.4), (School, 0, 0.3), (Workplace, 0, 0.3)],
                "「同窓会の案内を送りたいので」と言うと、電話に出た人があっさり教えてくれた",
                "「本人に確認します」と言われ、電話を切られた",
            ),
            "apo_call" => (
                if age >= 65 { 0.3 + 0.5 * (1.0 - c) } else { 0.1 + 0.15 * (1.0 - c) },
                &[(Family, 0, 0.9), (Routine, 0, 0.8), (Assets, 0, 0.7), (Health, 0, 0.3)],
                "「市の調査です」と名乗ると、家族構成や在宅時間を話してくれた",
                "不審がられて電話を切られた",
            ),
            "smishing" => (
                0.06 + 0.2 * (1.0 - c) + if age >= 65 || age <= 19 { 0.08 } else { 0.0 },
                &[(Name, 0, 0.9), (Address, 0, 0.9), (BirthDate, 0, 0.5), (Card, 1, 0.5)],
                "偽の再配達ページに氏名・住所などが入力された",
                "SMSのリンクは開かれなかった",
            ),
            "phishing" => (
                0.08 + 0.25 * (1.0 - c),
                &[(Password, 1, 1.0), (Name, 0, 0.8), (Address, 0, 0.7), (History, 0, 0.8), (Card, 0, 0.5), (Mobile, 0, 0.5)],
                "偽のログインページにパスワードが入力された。そのままログインして登録情報を盗み見た",
                "メールは無視された（迷惑メールに振り分けられたかもしれない）",
            ),
            "fake_campaign" => (
                0.08 + 0.35 * o * (1.0 - c),
                &[(Name, 0, 0.9), (Address, 0, 0.9), (Mobile, 0, 0.7), (BirthDate, 0, 0.5), (Email, 0, 0.6)],
                "「当選しました」のDMに、賞品の送り先として住所や氏名が送られてきた",
                "企画には反応がなかった",
            ),
            "quiz_app" => (
                0.15 + 0.45 * o,
                &[(BirthDate, 0, 0.8), (Pet, 0, 0.8), (Family, 9, 0.6), (School, 1, 0.6), (Hobby, 0, 0.5), (Idol, 0, 0.4)],
                "診断に回答してくれた。答えは「秘密の質問」にそのまま使えそうだ",
                "診断には回答してもらえなかった",
            ),
            "fake_job" => (
                if (16..=29).contains(&age) { 0.1 + 0.35 * (1.0 - c) } else { 0.03 },
                &[(Name, 0, 1.0), (Address, 0, 0.9), (BirthDate, 0, 0.9), (Mobile, 0, 0.9), (School, 0, 0.8), (Face, 9, 0.9), (IdDoc, 0, 0.8), (Email, 0, 0.6)],
                "応募があり、履歴書と身分証の画像が送られてきた",
                "応募はなかった",
            ),
            "trashing" => (
                0.65 * (1.0 - 0.6 * c),
                &[(Name, 0, 0.9), (Card, 0, 0.4), (Workplace, 0, 0.4), (Health, 0, 0.35), (History, 0, 0.5), (Family, 0, 0.5), (Income, 0, 0.3)],
                "ゴミ袋の中から宛名付きの郵便物や明細書、レシートが見つかった",
                "書類はシュレッダーにかけられていて読めなかった",
            ),
            _ => return Err("未対応の手口です".into()),
        };
        let cost = a.minutes;
        if self.alert_of(pid) >= 3 {
            return Ok(Out::new(false, cost, "相手は強く警戒していて、もう引っかからない"));
        }
        if !self.se_roll(pid, ai, base) {
            return Ok(Out::new(false, cost, ng_msg));
        }
        let mut out = Out::new(true, cost, ok_msg);
        self.se_results(t, &mut out, pid, ai, list, &[Parent::Fact(fid)]);
        Ok(out)
    }
}
