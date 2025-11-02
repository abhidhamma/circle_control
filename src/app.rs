use egui::{Color32, Pos2, Slider, Vec2, vec2};
// C++의 glm 대신 glam 크레이트 사용
use glam::{Vec3, vec3};

// Ray, Hit, Sphere, Raytracer Rust 구조체 정의

/// 광선 충돌 정보
struct Hit {
    /// 광선 시작점부터 충돌 지점까지의 거리 (음수: 충돌 없음)
    distance: f32,
    /// 충돌 지점 (3D 월드 좌표)
    point: Vec3,
    /// 충돌 지점의 법선 벡터 (normal)
    normal: Vec3,
}

/// 3D 공간의 광선
struct Ray {
    /// 광선 시작점
    origin: Vec3,
    /// 광선 방향 벡터(단위 벡터)
    direction: Vec3,
}

/// 3D 공간의 구
struct Sphere {
    center: Vec3,
    radius: f32,
    color: Vec3, // f32 벡터 색상이 계산에 편리
}

impl Sphere {
    /*
    IntersectionRayCollision

    레이트레이싱(광추적기)이 뭔지 그 틀을 알고
    3차원 구 그리기
    3차원 구를 그리기 위해서는 세가지를 구현해야한다
    1.광선과 구의 충돌
    2.조명효과
    3.원근투영

    그중에 오늘은 광선과 구의 충돌을 먼저 한다

    ---
    가상의 공간과 가상의 구를 만들고
    모니터 픽셀에서부터 수직으로 쏜 광선이 물체를 만났을때 그 색으로 픽셀의 색을 결정한다.
    그러면 눈으로 이 위치에 물체가 있다고 화면으로 볼 수 있게 된다
    컨트롤러에서 2차원 좌표를 받아서 3차원 좌표로 변환한다

    충돌(접점) 계산함수
    빛이 구를 통과하는 경우
    1)통과하지 않는 경우
    2)통과가 한번만 이뤄지는 경우
    3)통과가 두번 이뤄지는 경우

    ### 구
    구의 방정식(x, c는 3차원벡터)
    $||x - c||^2 = r^2$

    ### 광선
    직선의 방정식
    x = o + du
    x: 직선의 점들
    o: 광선의 시작점
    d: 선과의 거리
    u: 광선의 방향

    거리를 얼마나 가면 충돌을 하는지 d를 찾는것이 목표다

    ### 풀이
    접하는 지점을 찾으려면 구의 방정식의 x와 직선의 방정식 x가 같아지는 점을 찾으면 된다.

    x를 o+du로 치환하면
    $$||o+ du - c||^2 = r^2$$
    이 식을 d에 대해 전개하면
    d에 대한 2차방정식이 된다
    $$ad^2 + bd + c = 0$$
    이때
    a는 단위벡터니까 계산 안해도 되고 b와 c를 통해 근의공식으로 계산하면 된다

    계산해보면
    d = $-[u \cdot(o-c)] \pm \sqrt{\nabla}$
    $\nabla = (o-c)^2 - (||o-c||^2 - r^2)$
    이 공식을 코드로 구현하면 된다

    이 판별식 $\nabla$에 따라
    $\nabla < 0$: 충돌하지 않음
    $\nabla = 0$: 1점 충돌(구의 접선)
    $\nabla > 0$: 2점 충돌(구 통과)
    이므로
    $\nabla \geq 0$ 일때 구와의 거리 d를 계산하면 된다.

    0보다 작을 경우에는 d를 계산하지 않는다
    또한 $\nabla \geq 0$ 이더라도 d가 음수이면 충돌지점이 광선의 출발점 뒤에 있는것이므로 계산하지 않는다

     */
    fn intersect_ray_collision(&self, ray: &Ray) -> Hit {
        // origin - center 벡터, 자주 사용되므로 변수 저장
        let oc = ray.origin - self.center;

        // a = 1 (ray.direction은 단위 벡터)
        // 2차 방정식: t² + 2(d·oc)t + oc² - r² = 0
        // b/2 (half_b)를 사용하면 계산 간결화
        let half_b = ray.direction.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;

        // 판별식 (nabla) = (b/2)² - c (a=1 이므로)
        let discriminant = half_b * half_b - c;

        // 판별식이 0보다 작으면 실근 없음 (충돌 안 함)
        if discriminant < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
            };
        }

        // 두 실근(충돌 거리) 계산
        // 광선이 구에 들어가는 지점과 나가는 지점
        let sqrt_discriminant = discriminant.sqrt();
        let t1 = -half_b - sqrt_discriminant;
        let t2 = -half_b + sqrt_discriminant;

        // 두 근 중 더 작은 양수 값이 카메라에 가까운 충돌점
        let distance = if t1 >= 0.0 {
            t1
        } else if t2 >= 0.0 {
            t2
        } else {
            // 두 근 모두 음수이면 광선위 시작점 뒤쪽이므로 충돌 아님
            -1.0
        };

        if distance < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
            };
        }

        // 충돌 정보 계산 및 반환
        let point = ray.origin + ray.direction * distance;
        let normal = (point - self.center).normalize();
        Hit {
            distance,
            point,
            normal,
        }
    }
}

/// 레이트레이싱 계산 담당
struct Raytracer {
    width: i32,
    height: i32,
    sphere: Sphere,
}

impl Raytracer {
    fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            sphere: Sphere {
                center: vec3(0.0, 0.0, 0.5),
                radius: 0.4,
                color: vec3(1.0, 1.0, 1.0), // 흰색
            },
        }
    }

    /// 2D 스크린 좌표를 3D 월드 좌표로 변환
    fn transform_screen_to_world(&self, pos_screen: Vec2) -> Vec3 {
        let x_scale = 2.0 / (self.width - 1) as f32;
        let y_scale = 2.0 / (self.height - 1) as f32;
        let aspect = self.width as f32 / self.height as f32;

        // 3차원 공간으로 확장 (z좌표는 0.0으로 가정)
        vec3(
            (pos_screen.x * x_scale - 1.0) * aspect,
            -pos_screen.y * y_scale + 1.0,
            0.0,
        )
    }

    /// 주어진 광선을 추적해 최종 색상 계산
    fn trace_ray(&self, ray: &Ray) -> Vec3 {
        let hit = self.sphere.intersect_ray_collision(ray);

        if hit.distance < 0.0 {
            // 충돌하지 않으면 검은색 반환
            vec3(0.0, 0.0, 0.0)
        } else {
            // C++ 예제처럼 깊이(distance)를 곱해 입체감 표현
            self.sphere.color * hit.distance
        }
    }

    /// 모든 픽셀에 대해 레이트레이싱을 수행해 픽셀 버퍼를 채움
    fn render(&self, pixels: &mut [Color32]) {
        // rayon을 사용해 병렬 처리 (C++의 #pragma omp parallel for와 유사)
        use rayon::prelude::*;

        pixels.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
            let i = idx % self.width as usize;
            let j = idx / self.width as usize;

            let pos_world = self
                .transform_screen_to_world(vec2(i as f32, j as f32));

            // Orthographic projection (정투영): 모든 광선이 z축으로 평행하게 나아감
            let ray_dir = vec3(0.0, 0.0, 1.0);
            let pixel_ray = Ray {
                origin: pos_world,
                direction: ray_dir,
            };

            // 광선을 추적해 색상 계산
            let color_vec = self.trace_ray(&pixel_ray);

            // 계산된 색상(Vec3, 0.0~1.0)을 Color32(u8, 0~255)로 변환
            *pixel = Color32::from_rgb(
                (color_vec.x.clamp(0.0, 1.0) * 255.0) as u8,
                (color_vec.y.clamp(0.0, 1.0) * 255.0) as u8,
                (color_vec.z.clamp(0.0, 1.0) * 255.0) as u8,
            );
        });
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    // serde(skip): 저장/로드 상태에서 제외
    #[serde(skip)]
    raytracer: Raytracer,

    // UI 컨트롤과 직접 연결될 변수들
    center_x: f32,
    center_y: f32,
    center_z: f32,
    radius: f32,
    // egui의 color_edit_button_rgb는 [f32; 3] 타입을 사용
    color: [f32; 3],
}

mod consts {
    use egui::Color32;
    pub const MAX_RESOLUTION_X: i32 = 1280;
    pub const MAX_RESOLUTION_Y: i32 = 720;
    pub const DEFAULT_BACKGROUND_COLOR: Color32 =
        Color32::from_gray(30);
}

impl Default for TemplateApp {
    fn default() -> Self {
        let raytracer = Raytracer::new(
            consts::MAX_RESOLUTION_X,
            consts::MAX_RESOLUTION_Y,
        );
        Self {
            center_x: raytracer.sphere.center.x,
            center_y: raytracer.sphere.center.y,
            center_z: raytracer.sphere.center.z,
            radius: raytracer.sphere.radius,
            color: [
                raytracer.sphere.color.x,
                raytracer.sphere.color.y,
                raytracer.sphere.color.z,
            ],
            raytracer,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY)
                .unwrap_or_default();
        }
        Self::default()
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        // UI 컨트롤러 값을 Raytracer의 Sphere 데이터에 반영
        self.raytracer.sphere.center.x = self.center_x;
        self.raytracer.sphere.center.y = self.center_y;
        self.raytracer.sphere.center.z = self.center_z;
        self.raytracer.sphere.radius = self.radius;
        self.raytracer.sphere.color = Vec3::from_slice(&self.color);

        egui::SidePanel::left("control_panel").show(ctx, |ui| {
            ui.heading("Sphere Controls");
            ui.add(
                Slider::new(&mut self.center_x, -1.0..=1.0)
                    .text("Center X"),
            );
            ui.add(
                Slider::new(&mut self.center_y, -1.0..=1.0)
                    .text("Center Y"),
            );
            ui.add(
                Slider::new(&mut self.center_z, -1.0..=1.0)
                    .text("Center Z"),
            );
            ui.add(
                Slider::new(&mut self.radius, 0.0..=1.0)
                    .text("Radius"),
            );
            ui.label("Color:");
            // color_edit_button_rgb는 [f32; 3] 타입의 슬라이서를 사용
            ui.color_edit_button_rgb(&mut self.color);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let width = self.raytracer.width as usize;
            let height = self.raytracer.height as usize;

            // 1. 픽셀 데이터를 담을 버퍼 생성
            let mut pixels: Vec<Color32> = vec![
                    consts::DEFAULT_BACKGROUND_COLOR;
                    width * height
                ];

            // 2. Raytracer가 픽셀 버퍼를 채움 (CPU 렌더링)
            self.raytracer.render(&mut pixels);

            // 3. 픽셀 데이터로 이미지 생성
            let image = egui::ColorImage {
                size: [width, height],
                source_size: vec2(width as f32, height as f32), // 이 줄을 추가합니다.
                pixels,
            };

            // 4. ColorImage를 GPU 텍스처로 로드하고 화면에 그림
            let texture = ctx.load_texture(
                "sphere_canvas",
                image,
                egui::TextureOptions::NEAREST,
            );
            ui.image((texture.id(), texture.size_vec2()));
        });

        // UI 변경 시 화면을 다시 그리도록 요청
        ctx.request_repaint();
    }
}
