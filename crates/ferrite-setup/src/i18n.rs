use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

#[derive(Debug)]
pub struct Text {
    pub not_found: &'static str,
    pub pick: &'static str,
    pub write_failed: &'static str,
    pub loader_done: &'static str,
    pub installed: &'static str,
    pub updated: &'static str,
}

pub fn text() -> &'static Text {
    match unsafe { GetUserDefaultUILanguage() } & 0x3ff {
        0x19 => &RU,
        0x22 => &UK,
        0x04 => &ZH,
        0x0a => &ES,
        0x16 => &PT,
        0x07 => &DE,
        0x0c => &FR,
        0x11 => &JA,
        0x12 => &KO,
        0x1f => &TR,
        0x15 => &PL,
        _ => &EN,
    }
}

pub fn fill(template: &str, args: &[&str]) -> String {
    args.iter()
        .enumerate()
        .fold(template.to_owned(), |out, (i, a)| {
            out.replace(&format!("{{{i}}}"), a)
        })
}

const EN: Text = Text {
    not_found: "No Unity game found.\n\nPut the installer into the game folder (next to the game's .exe) and run it again, or pick the game folder when asked.",
    pick: "Select the game folder (the one with the game's .exe)",
    write_failed: "Could not write files:\n{0}\n\nClose the game and try again. If that does not help, run the installer as administrator.",
    loader_done: "Ferrite {0}.\n\nGame folder:\n{1}\n\nPut mods into Ferrite\\mods.\nThe installer will now delete itself.",
    installed: "installed",
    updated: "updated",
};

const RU: Text = Text {
    not_found: "Unity-игра не найдена.\n\nПоложите установщик в папку игры (рядом с .exe игры) и запустите снова или выберите папку игры, когда установщик спросит.",
    pick: "Выберите папку игры (ту, где лежит .exe игры)",
    write_failed: "Не удалось записать файлы:\n{0}\n\nЗакройте игру и попробуйте снова. Если не помогло — запустите установщик от имени администратора.",
    loader_done: "Ferrite {0}.\n\nПапка игры:\n{1}\n\nМоды кладите в Ferrite\\mods.\nУстановщик сейчас удалится.",
    installed: "установлен",
    updated: "обновлён",
};

const UK: Text = Text {
    not_found: "Unity-гру не знайдено.\n\nПокладіть інсталятор у папку гри (поруч із .exe гри) і запустіть знову або виберіть папку гри, коли інсталятор запитає.",
    pick: "Виберіть папку гри (ту, де лежить .exe гри)",
    write_failed: "Не вдалося записати файли:\n{0}\n\nЗакрийте гру і спробуйте ще раз. Якщо не допомогло — запустіть інсталятор від імені адміністратора.",
    loader_done: "Ferrite {0}.\n\nПапка гри:\n{1}\n\nМоди кладіть у Ferrite\\mods.\nІнсталятор зараз видалиться.",
    installed: "встановлено",
    updated: "оновлено",
};

const ZH: Text = Text {
    not_found: "未找到 Unity 游戏。\n\n请将安装程序放入游戏文件夹（与游戏 .exe 同目录）后重新运行，或在提示时选择游戏文件夹。",
    pick: "请选择游戏文件夹（包含游戏 .exe 的文件夹）",
    write_failed: "无法写入文件：\n{0}\n\n请关闭游戏后重试。如果仍然失败，请以管理员身份运行安装程序。",
    loader_done: "Ferrite {0}。\n\n游戏文件夹：\n{1}\n\n请将模组放入 Ferrite\\mods。\n安装程序将自动删除。",
    installed: "已安装",
    updated: "已更新",
};

const ES: Text = Text {
    not_found: "No se encontró ningún juego de Unity.\n\nColoca el instalador en la carpeta del juego (junto al .exe del juego) y ejecútalo de nuevo, o elige la carpeta del juego cuando se te pida.",
    pick: "Selecciona la carpeta del juego (la que contiene el .exe del juego)",
    write_failed: "No se pudieron escribir los archivos:\n{0}\n\nCierra el juego e inténtalo de nuevo. Si no funciona, ejecuta el instalador como administrador.",
    loader_done: "Ferrite {0}.\n\nCarpeta del juego:\n{1}\n\nPon los mods en Ferrite\\mods.\nEl instalador se eliminará ahora.",
    installed: "instalado",
    updated: "actualizado",
};

const PT: Text = Text {
    not_found: "Nenhum jogo Unity encontrado.\n\nColoque o instalador na pasta do jogo (ao lado do .exe do jogo) e execute-o novamente, ou escolha a pasta do jogo quando solicitado.",
    pick: "Selecione a pasta do jogo (a que contém o .exe do jogo)",
    write_failed: "Não foi possível gravar os arquivos:\n{0}\n\nFeche o jogo e tente novamente. Se não resolver, execute o instalador como administrador.",
    loader_done: "Ferrite {0}.\n\nPasta do jogo:\n{1}\n\nColoque os mods em Ferrite\\mods.\nO instalador será excluído agora.",
    installed: "instalado",
    updated: "atualizado",
};

const DE: Text = Text {
    not_found: "Kein Unity-Spiel gefunden.\n\nLege den Installer in den Spielordner (neben die .exe des Spiels) und starte ihn erneut, oder wähle den Spielordner, wenn du gefragt wirst.",
    pick: "Wähle den Spielordner (den mit der .exe des Spiels)",
    write_failed: "Dateien konnten nicht geschrieben werden:\n{0}\n\nSchließe das Spiel und versuche es erneut. Falls das nicht hilft, starte den Installer als Administrator.",
    loader_done: "Ferrite {0}.\n\nSpielordner:\n{1}\n\nMods kommen in Ferrite\\mods.\nDer Installer löscht sich jetzt selbst.",
    installed: "installiert",
    updated: "aktualisiert",
};

const FR: Text = Text {
    not_found: "Aucun jeu Unity trouvé.\n\nPlacez l'installateur dans le dossier du jeu (à côté du .exe du jeu) et relancez-le, ou choisissez le dossier du jeu lorsqu'il vous est demandé.",
    pick: "Choisissez le dossier du jeu (celui qui contient le .exe du jeu)",
    write_failed: "Impossible d'écrire les fichiers :\n{0}\n\nFermez le jeu et réessayez. Si cela ne suffit pas, lancez l'installateur en tant qu'administrateur.",
    loader_done: "Ferrite {0}.\n\nDossier du jeu :\n{1}\n\nPlacez les mods dans Ferrite\\mods.\nL'installateur va maintenant se supprimer.",
    installed: "installé",
    updated: "mis à jour",
};

const JA: Text = Text {
    not_found: "Unity ゲームが見つかりません。\n\nインストーラーをゲームフォルダー（ゲームの .exe と同じ場所）に置いてもう一度実行するか、表示されたらゲームフォルダーを選択してください。",
    pick: "ゲームフォルダー（ゲームの .exe があるフォルダー）を選択してください",
    write_failed: "ファイルを書き込めませんでした：\n{0}\n\nゲームを終了してからもう一度お試しください。解決しない場合は、管理者としてインストーラーを実行してください。",
    loader_done: "Ferrite を{0}。\n\nゲームフォルダー：\n{1}\n\nMOD は Ferrite\\mods に置いてください。\nインストーラーは自動的に削除されます。",
    installed: "インストールしました",
    updated: "更新しました",
};

const KO: Text = Text {
    not_found: "Unity 게임을 찾을 수 없습니다.\n\n설치 프로그램을 게임 폴더(게임 .exe가 있는 곳)에 넣고 다시 실행하거나, 요청 시 게임 폴더를 선택하세요.",
    pick: "게임 폴더(게임 .exe가 있는 폴더)를 선택하세요",
    write_failed: "파일을 쓸 수 없습니다:\n{0}\n\n게임을 종료하고 다시 시도하세요. 그래도 안 되면 관리자 권한으로 설치 프로그램을 실행하세요.",
    loader_done: "Ferrite가 {0}.\n\n게임 폴더:\n{1}\n\n모드는 Ferrite\\mods에 넣으세요.\n설치 프로그램은 이제 자동으로 삭제됩니다.",
    installed: "설치되었습니다",
    updated: "업데이트되었습니다",
};

const TR: Text = Text {
    not_found: "Unity oyunu bulunamadı.\n\nYükleyiciyi oyun klasörüne (oyunun .exe dosyasının yanına) koyup tekrar çalıştırın veya istendiğinde oyun klasörünü seçin.",
    pick: "Oyun klasörünü seçin (oyunun .exe dosyasının bulunduğu klasör)",
    write_failed: "Dosyalar yazılamadı:\n{0}\n\nOyunu kapatıp tekrar deneyin. Sorun devam ederse yükleyiciyi yönetici olarak çalıştırın.",
    loader_done: "Ferrite {0}.\n\nOyun klasörü:\n{1}\n\nModları Ferrite\\mods klasörüne koyun.\nYükleyici şimdi kendini silecek.",
    installed: "kuruldu",
    updated: "güncellendi",
};

const PL: Text = Text {
    not_found: "Nie znaleziono gry Unity.\n\nUmieść instalator w folderze gry (obok pliku .exe gry) i uruchom go ponownie albo wybierz folder gry, gdy instalator o to poprosi.",
    pick: "Wybierz folder gry (ten z plikiem .exe gry)",
    write_failed: "Nie udało się zapisać plików:\n{0}\n\nZamknij grę i spróbuj ponownie. Jeśli to nie pomoże, uruchom instalator jako administrator.",
    loader_done: "Ferrite {0}.\n\nFolder gry:\n{1}\n\nMody umieszczaj w Ferrite\\mods.\nInstalator zaraz się usunie.",
    installed: "zainstalowany",
    updated: "zaktualizowany",
};
