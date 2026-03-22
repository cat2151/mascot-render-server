# mascot-render-server

デスクトップマスコット（の簡易版）。Rustで書かれています。

## 特徴
- かんたんインストール。zipを置くだけ。
- らくらくエディット。TUIで直感的に着せ替えやポーズ変更ができます。
- ゆかいなリアクション。頭をクリックすると…
- 簡易版です。機能は少ないので遊び心が必要です。

## install

```
cargo install --force --git https://github.com/cat2151/mascot-render-server mascot-render-server psd-viewer-tui
```

## 準備

Rustが必要です。

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

- 詳しい機能は画面のhelpを参照ください

## 設定
- mascot-render-server.toml
    - transparent_background_click_through
        - trueにすると、非常に重たくなるかわりに、「なにもない空間をドラッグして混乱」を減らせます。
    - flash_blue_background_on_transparent_input
        - trueにすると、なにもない空間をクリックやドラッグしようとしたときに1秒ブルーバックにして知らせます。

## アーキテクチャ
- モジュラー
    - 再利用性を重視し、責務ごとの小さなクレートに分割して実装してあります。
- Sixel
    - デスクトップマスコットが万一動かない場合のフォールバック用で、terminalにマスコットのpreviewが表示されます。
- format
    - ghostやshellなどの管理formatの実現はできていません
        - 現状、坂本アヒル様作ずんだもん立ち絵素材でのみtestをしています
            - tomlファイルを書き換えれば汎用で使える可能性はありますが未確認です

## vendor/ について

`vendor/rawpsd` は `rawpsd` ライブラリの不具合に対して、AI 支援で修正を入れた vendored copy です。

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
