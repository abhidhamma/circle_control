use egui::{Color32, Slider, Vec2, vec2};
use glam::{Vec3, vec3};
use std::any::Any;
use std::sync::Arc;

// Send + Sync는 여러 스레드에서 안전하게 공유 가능하도록 하는 제약 조건
// Any 트레잇을 상속받아 다운캐스팅이 가능하도록 하기
trait Object: Send + Sync + Any {
    fn check_ray_collision(&self, ray: &Ray) -> Hit;
    fn ambient(&self) -> Vec3;
    fn diffuse(&self) -> Vec3;
    fn specular(&self) -> Vec3;
    fn alpha(&self) -> f32;
    // 다운캐스팅을 위한 as_any 메소드
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// 광추적 충돌정보
struct Hit {
    distance: f32,
    point: Vec3,
    normal: Vec3,
    // 충돌한 객체의 재질 정보를 가져오기 위해 Arc<dyn Object>를 저장
    object: Option<Arc<dyn Object>>,
}

// 광선
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

// 구
struct Sphere {
    center: Vec3,
    radius: f32,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
}

impl Object for Sphere {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        let oc = ray.origin - self.center;
        let b = 2.0 * ray.direction.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = b * b - 4.0 * c;

        if discriminant < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                object: None,
            };
        }

        let sqrt_discriminant = discriminant.sqrt();
        let t1 = (-b - sqrt_discriminant) / 2.0;
        let t2 = (-b + sqrt_discriminant) / 2.0;

        let distance = if t1 >= 0.0 && (t1 < t2 || t2 < 0.0) {
            t1
        } else if t2 >= 0.0 {
            t2
        } else {
            -1.0
        };

        if distance < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                object: None,
            };
        }

        let point = ray.origin + ray.direction * distance;
        let normal = (point - self.center).normalize();
        Hit {
            distance,
            point,
            normal,
            object: None, // 실제 객체 정보는 find_closest_collision에서 채우기
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn ambient(&self) -> Vec3 {
        self.amb
    }
    fn diffuse(&self) -> Vec3 {
        self.diff
    }
    fn specular(&self) -> Vec3 {
        self.spec
    }
    fn alpha(&self) -> f32 {
        self.alpha
    }
}

struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
}

impl Object for Triangle {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        let edge1 = self.v1 - self.v0;
        let edge2 = self.v2 - self.v0;
        let h = ray.direction.cross(edge2);
        let a = edge1.dot(h);

        if a > -1e-6 && a < 1e-6 {
            // 광선이 평면과 평행
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                object: None,
            };
        }

        let f = 1.0 / a;
        let s = ray.origin - self.v0;
        let u = f * s.dot(h);

        if u < 0.0 || u > 1.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                object: None,
            };
        }

        let q = s.cross(edge1);
        let v = f * ray.direction.dot(q);

        if v < 0.0 || u + v > 1.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                object: None,
            };
        }

        let t = f * edge2.dot(q);
        if t > 1e-6 {
            // 광선과의 교점
            let point = ray.origin + ray.direction * t;
            let normal = edge1.cross(edge2).normalize();
            Hit {
                distance: t,
                point,
                normal,
                object: None,
            }
        } else {
            Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                object: None,
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn ambient(&self) -> Vec3 {
        self.amb
    }
    fn diffuse(&self) -> Vec3 {
        self.diff
    }
    fn specular(&self) -> Vec3 {
        self.spec
    }
    fn alpha(&self) -> f32 {
        self.alpha
    }
}

/*
    사각형 그리기(Square 구조체)
    Square는 두 개의 Triangle 객체로 사각형을 그림(has-a 관계)
    Square 자체가 Object 트레잇을 구현하도록 하여, 외부에서는 이것이 사각형인지 삼각형인지 신경 쓸 필요 없이
    check_ray_collision 함수를 호출할 수 있음(composition).
    또한 square는 삼각형의 함수를 재사용해서 자신은 판단하는 로직만 작성함(deligation)

    composition의 근본적인 이점: 유연성(낮은 결합도와 명확한 역할과 책임을 주면 유연해진다)
    composition(구성)의 뉘앙스는 낮은 결합도를 통해 교체가능성을 확보한다는 조립의 의미
    또한 유연함의 비교대상은 상속이고 상속(is-a관계)보다 유연해진다는 것이다.
    상속(is-a a는 b의 한 종류다): 기존 객체를 확장하여 더 특수화된 객체를 만드는것
    구성(has-a, a는 b를 가지고 있다): 각 객체를 독립적인 부품으로 만들고 그것들을 조립해서 더 큰 객체로 만들기

    1.재사용성: 자연스럽게 코드중복이 줄어듬
    2.낮은 결합도:
    외부에서 봤을때 triangle과 square는 모두 같은 check_ray_collision을 사용하고 있기 때문에 내부를 알 필요가 없어서 의존성이 낮아진다.
    또한 외부의존성이 없기 떄문에 수정이나 교체가 용이하다

    3.명확한 역할과 책임:
    triangle: 세 점으로 이루어진 삼각형과 광선의 충돌을 계산하는 책임
    square: 두개의 triangle을 관리하고 충돌검사 요청이 오면 일을 위임하고 결과를 종합하는 책임
    raytraccer: 화면의 모든 픽셀에 대해 광선을 쏘고 충돌검사를 요청해서 최종 색상을 결정하는 책임

    독립적이고 완전한 기능: triangle, square의 check_ray_collision
    표준화된 연결부: 같은 규격을 통해 연결될 수 있게 함 Object 트레잇은 objects벡터에 포함됨
*/

struct Square {
    triangle1: Triangle,
    triangle2: Triangle,
}

impl Object for Square {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        // 두 삼각형에 대해 각각 충돌 검사
        let hit1 = self.triangle1.check_ray_collision(ray);
        let hit2 = self.triangle2.check_ray_collision(ray);

        // 두 삼각형 모두와 충돌했다면, 더 가까운 쪽의 충돌 정보를 반환
        if hit1.distance >= 0.0 && hit2.distance >= 0.0 {
            if hit1.distance < hit2.distance {
                hit1
            } else {
                hit2
            }
        }
        // 한 쪽만 충돌했다면, 그 충돌 정보를 반환
        else if hit1.distance >= 0.0 {
            hit1
        // hit2가 충돌했거나, 둘 다 충돌하지 않은 경우 (-1.0)
        } else {
            hit2
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    // Square의 재질은 첫 번째 삼각형의 재질을 대표로 사용
    fn ambient(&self) -> Vec3 {
        self.triangle1.ambient()
    }
    fn diffuse(&self) -> Vec3 {
        self.triangle1.diffuse()
    }
    fn specular(&self) -> Vec3 {
        self.triangle1.specular()
    }
    fn alpha(&self) -> f32 {
        self.triangle1.alpha()
    }
}

struct Light {
    pos: Vec3,
}

struct Raytracer {
    width: i32,
    height: i32,
    objects: Vec<Arc<dyn Object>>,
    light: Light,
    sphere_center: Vec3, // UI에서 구 위치를 조작하기 위해 추가
}

impl Raytracer {
    fn new(width: i32, height: i32) -> Self {
        let sphere_center = vec3(0.0, 0.0, 0.6);
        let sphere1 = Arc::new(Sphere {
            center: sphere_center,
            radius: 0.4,
            amb: vec3(0.2, 0.0, 0.0),
            diff: vec3(1.0, 0.1, 0.1),
            spec: vec3(1.5, 1.5, 1.5),
            alpha: 50.0,
        });

        // 사각형 바닥을 정의합니다.
        let floor = Arc::new(Square {
            triangle1: Triangle {
                v0: vec3(-2.0, -1.0, 0.0),
                v1: vec3(-2.0, -1.0, 4.0),
                v2: vec3(2.0, -1.0, 4.0),
                amb: vec3(0.2, 0.2, 0.2),
                diff: vec3(0.8, 0.8, 0.8),
                spec: vec3(1.0, 1.0, 1.0),
                alpha: 50.0,
            },
            triangle2: Triangle {
                v0: vec3(-2.0, -1.0, 0.0),
                v1: vec3(2.0, -1.0, 4.0),
                v2: vec3(2.0, -1.0, 0.0),
                amb: vec3(0.2, 0.2, 0.2),
                diff: vec3(0.8, 0.8, 0.8),
                spec: vec3(1.0, 1.0, 1.0),
                alpha: 50.0,
            },
        });

        Self {
            width,
            height,
            objects: vec![sphere1, floor],
            light: Light {
                pos: vec3(0.0, 1.0, 0.2),
            },
            sphere_center,
        }
    }

    fn find_closest_collision(&self, ray: &Ray) -> Hit {
        let mut closest_d = f32::MAX;
        let mut closest_hit = Hit {
            distance: -1.0,
            point: Vec3::ZERO,
            normal: Vec3::ZERO,
            object: None,
        };

        for object in &self.objects {
            let mut hit = object.check_ray_collision(ray);
            if hit.distance >= 0.0 && hit.distance < closest_d {
                closest_d = hit.distance;
                hit.object = Some(object.clone());
                closest_hit = hit;
            }
        }
        closest_hit
    }

    fn transform_screen_to_world(&self, pos_screen: Vec2) -> Vec3 {
        let x_scale = 2.0 / (self.width - 1) as f32;
        let y_scale = 2.0 / (self.height - 1) as f32;
        let aspect = self.width as f32 / self.height as f32;

        vec3(
            (pos_screen.x * x_scale - 1.0) * aspect,
            -pos_screen.y * y_scale + 1.0,
            0.0,
        )
    }

    // 그림자 계산이 추가된 trace_ray 함수
    fn trace_ray(&self, ray: &Ray) -> Vec3 {
        let hit = self.find_closest_collision(ray);

        if let Some(obj) = hit.object {
            // 1. 기본 색상(Ambient)으로 시작
            let mut color = obj.ambient();

            // 2. 그림자 광선(Shadow Ray)을 생성
            //    - 시작점: 현재 충돌 지점(hit.point). 부동소수점 오차를 피하기 위해 빛 방향으로 살짝 이동시킵니다.
            //    - 방향: 충돌 지점에서 광원을 향하는 방향.
            let dir_to_light =
                (self.light.pos - hit.point).normalize();
            let shadow_ray = Ray {
                origin: hit.point + dir_to_light * 1e-4,
                direction: dir_to_light,
            };

            // 3. 그림자 광선으로 충돌 검사
            //    find_closest_collision 결과의 distance가 음수이면, 광원까지 가는 길에 아무것도 없다는 의미입니다.
            //    즉, 그림자가 지지 않은 상태입니다.
            if self.find_closest_collision(&shadow_ray).distance < 0.0
            {
                // 4. 그림자가 지지 않았다면, Diffuse와 Specular 색상을 계산하여 더하기
                let diff = hit.normal.dot(dir_to_light).max(0.0);
                let reflect_dir =
                    2.0 * hit.normal.dot(dir_to_light) * hit.normal
                        - dir_to_light;
                let specular = (-ray.direction)
                    .dot(reflect_dir)
                    .max(0.0)
                    .powf(obj.alpha());

                color +=
                    obj.diffuse() * diff + obj.specular() * specular;
            }

            // 최종 계산된 색상을 반환함(그림자일때는 ambient(주변광)만 있음)
            color
        } else {
            // 광선이 아무 물체와도 부딪히지 않으면 검은색을 반환
            vec3(0.0, 0.0, 0.0)
        }
    }

    fn render(&mut self, pixels: &mut [Color32]) {
        use rayon::prelude::*;

        // UI에서 변경된 구의 위치를 실제 객체에 반영합니다.
        // objects 벡터에서 Sphere를 찾아 업데이트합니다.
        if let Some(sphere_arc) = self.objects.get_mut(0) {
            if let Some(sphere) =
                Arc::get_mut(sphere_arc).and_then(|obj| {
                    (obj as &mut dyn Object)
                        .as_any_mut()
                        .downcast_mut::<Sphere>()
                })
            {
                sphere.center = self.sphere_center;
            }
        }

        let eye_pos = vec3(0.0, 0.0, -1.5);

        pixels.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
            let i = idx % self.width as usize;
            let j = idx / self.width as usize;

            let pos_world = self
                .transform_screen_to_world(vec2(i as f32, j as f32));
            let ray_dir = (pos_world - eye_pos).normalize();
            let pixel_ray = Ray {
                origin: eye_pos,
                direction: ray_dir,
            };

            let color_vec = self.trace_ray(&pixel_ray);

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
    #[serde(skip)]
    raytracer: Raytracer,

    // UI 컨트롤을 위한 변수들
    light_x: f32,
    light_y: f32,
    light_z: f32,
    sphere_x: f32,
    sphere_y: f32,
    sphere_z: f32,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let raytracer = Raytracer::new(1280, 720);
        Self {
            light_x: raytracer.light.pos.x,
            light_y: raytracer.light.pos.y,
            light_z: raytracer.light.pos.z,
            sphere_x: raytracer.sphere_center.x,
            sphere_y: raytracer.sphere_center.y,
            sphere_z: raytracer.sphere_center.z,
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
        // UI 값들을 Raytracer 객체에 반영
        self.raytracer.light.pos =
            vec3(self.light_x, self.light_y, self.light_z);
        self.raytracer.sphere_center =
            vec3(self.sphere_x, self.sphere_y, self.sphere_z);

        egui::SidePanel::left("control_panel").show(ctx, |ui| {
            ui.heading("Controls");
            ui.separator();
            ui.label("Light Position");
            ui.add(
                Slider::new(&mut self.light_x, -2.0..=2.0).text("X"),
            );
            ui.add(
                Slider::new(&mut self.light_y, -2.0..=2.0).text("Y"),
            );
            ui.add(
                Slider::new(&mut self.light_z, -2.0..=2.0).text("Z"),
            );
            ui.separator();
            ui.label("Sphere Position");
            ui.add(
                Slider::new(&mut self.sphere_x, -1.0..=1.0).text("X"),
            );
            ui.add(
                Slider::new(&mut self.sphere_y, -1.0..=1.0).text("Y"),
            );
            ui.add(
                Slider::new(&mut self.sphere_z, -1.0..=1.0).text("Z"),
            );
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let available_size = ui.available_size();
            self.raytracer.width = available_size.x as i32;
            self.raytracer.height = available_size.y as i32;

            let width = self.raytracer.width as usize;
            let height = self.raytracer.height as usize;

            let mut pixels: Vec<Color32> =
                vec![Color32::BLACK; width * height];

            self.raytracer.render(&mut pixels);

            let image = egui::ColorImage::from_rgba_unmultiplied(
                [width, height],
                bytemuck::cast_slice(&pixels),
            );

            let texture = ctx.load_texture(
                "raytrace_canvas",
                image,
                egui::TextureOptions::NEAREST,
            );
            ui.image((texture.id(), texture.size_vec2()));
        });

        ctx.request_repaint();
    }
}
