# mascot-render-server

デスクトップマスコット（の簡易版）。Rustで書かれています。

## 特徴
- かんたんインストール。zipを置くだけ。
- らくらくエディット。TUIで直感的に着せ替えやポーズ変更ができます。
- ゆかいなリアクション。頭をクリックすると…
- しゃべる : [voicevox-playground-tui](https://github.com/cat2151/voicevox-playground-tui/blob/main/README.ja.md)
- 全員集合 : TUIでfavorites登録したメンバーの立ち絵を並べて遊べます。全員並べることも可能。簡易版
- 簡易版です。機能は少ないので遊び心が必要です。

## install

Rustが必要です。

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui mascot-render-status-tui
```

## 準備

次の3つのzipファイルを `C:/Users/<YOUR NAME>/AppData/Local/mascot-render-server/assets/zip/` に配置してください。

坂本アヒル様作
- `ずんだもん立ち絵素材2.3.zip`
- `ずんだもん立ち絵素材V3.2.zip`
- `ずんだもん立ち絵素材改1.1.1.zip`

## 実行

```
psd-viewer-tui
```

- 以下が自動で行われます：
    - zipファイルの展開
    - zipに入っているpsdファイルの分析
    - psdに入っているレイヤーの分析
    - デスクトップマスコットの表示
        - デフォルトのレイヤーで表示されます

- 服装やポーズのレイヤーを変更すると見た目が変わります
- TUIでお気に入りを登録しておくと、server側がキャッシュを参照して1分ごとに別のお気に入りへshuffle再生します

- cat-smf-editorの設定`Ctrl+, → b`／パレットの「ずんだもん演奏連動」をONにすると、表示中のずんだもんが演奏の四分音符ごとに通常のクリック反応を基にした演奏専用の伸縮（`musical_squash_bounce`設定。既定は500ms・上下移動3px・周期数1・減衰2・横0.1・縦0.05）を先頭から再生し、対象の歌唱音符区間で口画像を既定の4 fpsで切り替えます。伸縮の長さは変えず、次の拍で途中の動きを打ち切って再開します（120 BPMでは500msごと）。テンポ変更・途中再生・ループに追従します。口画像は既存の遅延生成を使い、準備中もeditorの演奏は進みます。連動のON/OFFはeditorが保存し、再起動時に復元します。停止・OFF・editor終了・通信断ではserverの演奏状態を解除し、姿・配置・server設定は保存し直しません。SMPTE divisionの拍同期は対象外です。

- 詳しい機能は画面のhelpを参照ください

## 設定
- mascot-render-server.toml
    - always_idle_sink
        - 初期値は `true` です。
        - trueにすると、UX検証用に小さな IdleSink 呼吸風モーションを常時再生します。
    - always_bend
        - 初期値は `true` です。
        - trueにすると、UX検証用の左右 bend を常時再生します。
    - bend
        - `amplitude_ratio` で bend 幅をマスコット画像幅に対する比率で指定できます。デフォルトは `0.0075` です。
    - musical_squash_bounce
        - 演奏連動専用の伸縮です。`squash_bounce`（通常クリック時）とは独立しています。
        - `duration_ms`（動きの長さ）、`amplitude_px`（上下移動量）、`cycles`（周期数）、`damping`（減衰）、`squash_amount`（横の広がり量）、`stretch_amount`（縦のつぶれ量）を直接編集できます。`algorithm` は現在 `squash_stretch` の1種類です。
        - TOMLを保存すると自動で再読込します。毎拍先頭から再生するため、`duration_ms` が拍間隔より長ければ途中で打ち切ります。
    - idle_sink
        - always_idle_sink 専用の IdleSink 呼吸設定です。
        - デフォルトでは通常の squash_bounce より穏やかに沈み込み・持ち上がり、まばたき interval の中央値のゆらぎに合わせて少しずつテンポが変わります。
        - `sink_amount` と `lift_amount` で、呼気側と吸気側の pose を個別に調整できます。

## アーキテクチャ
- モジュラー
    - 再利用性を重視し、責務ごとの小さなクレートに分割して実装してあります。
- server
    - localhostの`POST /playback`は最新の演奏位置・拍速度・歌唱有効状態を保持し、750msの更新期限で解除します。口の開閉周期は連続した状態更新でリセットしません。既存の単発`/timeline`と別の一時状態であり、保存形式は変更しません。
    - visual-render-serverに徹する。セリフとmotionのオーケストレーション等の責務は、別の上位階層のアプリに持たせる考えです。
- Sixel
    - デスクトップマスコットが万一動かない場合のフォールバック用で、terminalにマスコットのpreviewが表示されます。
- PSDTool
    - [PSDTool](https://oov.github.io/psdtool/manual.html)の拡張フォーマット「ラジオボタン化」「強制表示化」に対応し、快適なeditを実現しています。
- format
    - ghostやshellなどの管理formatの実現はできていません
        - 現状、坂本アヒル様作ずんだもん立ち絵素材でのみtestをしています
            - tomlファイルを書き換えれば汎用で使える可能性はありますが未確認です

## vendor/ について

- `vendor/rawpsd` は [rawpsd](https://github.com/wareya/rawpsd-rs) ライブラリの不具合に対して、AI支援で修正を入れた vendored copy です。
- mascot-render-serverで取り扱うpsdを読もうとするとpanicしていたので、そこをCodex CLIに修正させたものです。
- CIの都合でリファクタリングしました。[PR 17](https://github.com/cat2151/mascot-render-server/pull/17#pullrequestreview-3988754980)

## 前提
- 自分用のアプリですので、他の人が使うことを想定していません。似たような機能がほしいときはcloneや自作をおすすめします。
- 頻繁に破壊的変更を行います。万一誰かが関連機能を作ったとしても翌日には使えなくなっているかもしれません。

## このアプリが目指すもの
- PoC。Codex CLI（Codex Plus 30日間無料お試し版）で自分用にあると助かるアプリが作れることを実証する（実証した）
- psd。Rustで楽にpsdを扱えること
- デスクトップマスコット。Rustで楽にデスクトップマスコットを実現できること
- 目パチと口パク
- server。HTTP REST APIで、楽に他のアプリからデスクトップマスコットを操作できること

## 目指さないもの（スコープ外）
- 新たな高機能汎用デスクトップマスコット共通規格の策定、それに向けたガバナンス体制の整備、継続的な運営
- サポート。要望や提案に応える
