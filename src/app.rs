// ------------------------------------------------------------------
// 아래 내용을 'src/app.rs' 파일 전체에 덮어쓰기 하세요.
// ------------------------------------------------------------------

// 1. 원 그리기에 필요한 use 구문들을 추가합니다.
use egui::{Color32, Pos2, Slider, Stroke};

// 템플릿의 `TemplateApp` 구조체를 사용합니다.
// #[serde(default)]는 앱을 껐다 켜도 슬라이더 값을 기억하게 해주는 기능입니다.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    // 2. 여기에 우리가 만들었던 원의 속성들을 추가합니다.
    center: Pos2,
    radius: f32,
    color: Color32,
}

// 3. 앱이 처음 켜질 때의 기본값을 설정합니다.
impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // 원의 기본값
            center: Pos2::new(400.0, 300.0),
            radius: 100.0,
            color: Color32::from_rgb(255, 0, 0), // 빨간색
        }
    }
}

// 4. `TemplateApp`의 `new` 함수 (앱 생성자)
// (템플릿에 이미 있는 내용이므로 그대로 둡니다.)
impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 저장된 상태를 불러오거나, 없으면 기본값을 사용합니다.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Self::default()
    }
}

// 5. `eframe::App` 트레이트를 구현합니다.
impl eframe::App for TemplateApp {
    // 6. (선택) 앱을 끌 때 상태를 저장하는 함수
    // (템플릿에 이미 있는 내용이므로 그대로 둡니다.)
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    // 7. (⭐️핵심⭐️) 매 프레임마다 호출되는 update 함수
    // 템플릿의 기본 내용을 지우고, '원 그리기' 로직으로 덮어씁니다.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- 1. ImGui/egui 슬라이더 UI 그리기 ---
        egui::Window::new("Circle Controls").show(ctx, |ui| {
            ui.label("Center:");
            ui.add(Slider::new(&mut self.center.x, 0.0..=800.0).text("X"));
            ui.add(Slider::new(&mut self.center.y, 0.0..=600.0).text("Y"));
            ui.add(Slider::new(&mut self.radius, 0.0..=500.0).text("Radius"));
            ui.label("Color:");
            ui.color_edit_button_srgba(&mut self.color);
        });

        // --- 2. 원 그리기 ---
        // UI 창 뒤쪽(배경)에 그림을 그립니다.
        let painter = ctx.layer_painter(egui::LayerId::background());

        // 채워진 원 그리기
        painter.circle_filled(self.center, self.radius, self.color);

        // 원 테두리 그리기
        painter.circle_stroke(
            self.center,
            self.radius,
            Stroke::new(2.0, Color32::WHITE), // 2픽셀 두께 흰색 테두리
        );
    }
}
