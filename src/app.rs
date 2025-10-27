use egui::{Color32, Pos2, Slider, Stroke, Vec2};

// eframe TemplateApp 구조체
// #[serde(default)]는 앱을 껐다 켜도 슬라이더 값을 기억하게 해주는 기능
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    center: Pos2,
    radius: f32,
    color: Color32,
}

// 상수
mod consts {
    use egui::Color32;

    // 최대값
    pub const MAX_RESOLUTION_X: f32 = 1280.0;
    pub const MAX_RESOLUTION_Y: f32 = 720.0;
    pub const MAX_RADIUS: f32 = 500.0;

    // 원의 기본값. 가운데 위치에 반지름 100인 빨간색 원
    pub const DEFAULT_RESOLUTION_X: f32 = MAX_RESOLUTION_X / 2.0;
    pub const DEFAULT_RESOLUTION_Y: f32 = MAX_RESOLUTION_Y / 2.0;
    pub const DEFAULT_RADIUS: f32 = MAX_RADIUS / 5.0;
    pub const DEFAULT_COLOR: Color32 = Color32::RED;
    pub const DEFAULT_BACKGROUND_COLOR: Color32 = Color32::BLACK;
}

// 앱이 처음 켜질 때의 기본값
impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            center: Pos2::new(consts::DEFAULT_RESOLUTION_X, consts::DEFAULT_RESOLUTION_Y),
            radius: consts::DEFAULT_RADIUS,
            color: consts::DEFAULT_COLOR, // 빨간색
        }
    }
}

// `TemplateApp`의 `new` 함수 (앱 생성자)
impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 저장된 상태를 불러오고 없으면 기본값을 사용
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Self::default()
    }
}

// eframe::App 트레이트
impl eframe::App for TemplateApp {
    // 앱을 끌 때 상태를 저장하는 함수
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    // 매 프레임마다 호출되는 update 함수
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 컨트롤러 UI
        egui::SidePanel::left("control_panel").show(ctx, |ui| {
            ui.heading("Circle Controls");

            ui.label("Center X:");
            ui.add(Slider::new(&mut self.center.x, 0.0..=consts::MAX_RESOLUTION_X).text("X"));

            ui.label("Center Y:");
            ui.add(Slider::new(&mut self.center.y, 0.0..=consts::MAX_RESOLUTION_Y).text("Y"));

            ui.add(Slider::new(&mut self.radius, 0.0..=consts::MAX_RADIUS).text("Radius"));

            ui.label("Color:");
            ui.color_edit_button_srgba(&mut self.color);
        });

        // 원 그리는 캔버스
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            let painter = ui.painter();

            // 2-1. 처음에 캔버스 전체를 검은색 배경으로 채움
            painter.rect_filled(rect, 0.0, consts::DEFAULT_BACKGROUND_COLOR);

            // 2-2. 원 그리기(원의 중심점 좌표를 Panel의 상대적인 위치로 조정
            let canvas_center = rect.left_top() + Vec2::new(self.center.x, self.center.y);

            // 채워진 원 그리기(내부적으로 중심점과 벡터의 거리가 반지름보다 작으면 픽셀을 채우는 작업)
            painter.circle_filled(canvas_center, self.radius, self.color);

            // 원 테두리 그리기
            painter.circle_stroke(canvas_center, self.radius, Stroke::new(2.0, Color32::WHITE));
        });
    }
}
