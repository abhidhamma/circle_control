// 원 그리기에 필요한 use 구문들을 추가합니다.
use egui::{Color32, Pos2, Slider, Stroke, Vec2};

// 템플릿의 `TemplateApp` 구조체
// #[serde(default)]는 앱을 껐다 켜도 슬라이더 값을 기억하게 해주는 기능
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    center: Pos2,
    radius: f32,
    color: Color32,
}

// 앱이 처음 켜질 때의 기본값
impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // 원의 기본값. X 400, Y 300 위치에 반지름 100인 빨간색 원
            center: Pos2::new(640.0, 400.0),
            radius: 100.0,
            color: Color32::from_rgb(255, 0, 0), // 빨간색
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

// `eframe::App` 트레이트를 구현합니다.
impl eframe::App for TemplateApp {
    // 앱을 끌 때 상태를 저장하는 함수
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    // 매 프레임마다 호출되는 update 함수
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- 1. 컨트롤러 UI (SidePanel로 분리) ---
        egui::SidePanel::left("control_panel").show(ctx, |ui| {
            ui.heading("Circle Controls");

            // X Center Slider
            ui.label("Center X:");
            // 가로해상도 1280
            ui.add(Slider::new(&mut self.center.x, 0.0..=1280.0).text("X"));

            // Y Center Slider
            ui.label("Center Y:");
            // 세로해상도 720
            ui.add(Slider::new(&mut self.center.y, 0.0..=720.0).text("Y"));

            // Radius Slider
            ui.add(Slider::new(&mut self.radius, 0.0..=500.0).text("Radius (반지름)"));

            // Color Picker
            ui.label("Color:");
            ui.color_edit_button_srgba(&mut self.color);
        });

        // --- 2. 원 그리기 캔버스 (CentralPanel) ---
        egui::CentralPanel::default().show(ctx, |ui| {
            // 이 CentralPanel 영역을 캔버스로 사용
            let rect = ui.available_rect_before_wrap();
            let painter = ui.painter();

            // 2-1. 처음에 모두 같은 색으로 초기화
            // 캔버스 전체를 검은색 배경으로 채움
            painter.rect_filled(
                rect,
                0.0,            // 코너 반경 없음
                Color32::BLACK, // 초기화 색상 (모두 같은 색)
            );

            // 2-2. 원 그리기 (벡터 거리를 이용한 IsInside 로직의 egui 구현)
            // egui의 circle_filled 함수는 내부적으로 중심점과의 벡터 거리를
            // 반지름과 비교하여 픽셀을 채우는 작업을 GPU를 통해 효율적으로 수행합니다.

            // 원의 중심점 좌표를 Panel의 상대적인 위치로 조정합니다.
            // (CentralPanel의 왼쪽 상단 좌표 + 저장된 center 위치)
            let canvas_center = rect.left_top() + Vec2::new(self.center.x, self.center.y);

            // 채워진 원 그리기
            painter.circle_filled(canvas_center, self.radius, self.color);

            // 원 테두리 그리기
            painter.circle_stroke(
                canvas_center,
                self.radius,
                Stroke::new(2.0, Color32::WHITE), // 2픽셀 두께 흰색 테두리
            );
        });
    }
}
