use eframe::egui;
use egui::{Color32, TextureOptions};
use glam::{Vec2, Vec3, vec2, vec3};
use image::GenericImageView;
use rayon::prelude::*;
use std::sync::Arc;

/*
 * Object 트레잇: 씬(Scene)에 포함될 수 있는 모든 객체의 공통 인터페이스
 * Send + Sync: 여러 스레드에서 안전하게 공유 가능 (rayon 병렬 처리용)
*/
trait Object: Send + Sync {
    fn check_ray_collision(&self, ray: &Ray) -> Hit;
    fn ambient(&self) -> Vec3;
    fn diffuse(&self) -> Vec3;
    fn specular(&self) -> Vec3;
    fn alpha(&self) -> f32;
    fn reflection(&self) -> f32;
    fn transparency(&self) -> f32;
    fn amb_texture(&self) -> Option<Arc<Texture>>;
    fn diff_texture(&self) -> Option<Arc<Texture>>;
}

// 광선 충돌 정보를 담는 구조체
// C++: Hit 클래스
struct Hit {
    distance: f32,
    point: Vec3,
    normal: Vec3,
    uv: Vec2, // 텍스처 좌표
    object: Option<Arc<dyn Object>>,
}

// 광선을 나타내는 구조체
// C++: Ray 클래스
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

// 구(Sphere)를 나타내는 구조체
// C++: Sphere 클래스
struct Sphere {
    center: Vec3,
    radius: f32,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
    reflection: f32,
    transparency: f32,
}

// Sphere에 대한 Object 트레잇 구현
impl Object for Sphere {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        let oc = ray.origin - self.center;
        let a = ray.direction.length_squared();
        let b = 2.0 * ray.direction.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                uv: Vec2::ZERO,
                object: None,
            };
        }

        let sqrt_discriminant = discriminant.sqrt();
        let t1 = (-b - sqrt_discriminant) / (2.0 * a);
        let t2 = (-b + sqrt_discriminant) / (2.0 * a);

        // C++ 코드에서는 min, max를 사용했지만, 물체 안에서 시작하는 광선을 고려하여
        // 먼저 t1 (더 가까운 점)이 양수인지 확인하고, 아니면 t2를 확인
        let distance = if t1 >= 0.0 {
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
                uv: Vec2::ZERO,
                object: None,
            };
        }

        let point = ray.origin + ray.direction * distance;
        let normal = (point - self.center).normalize();
        Hit {
            distance,
            point,
            normal,
            uv: Vec2::ZERO, // 구는 텍스처 좌표 없음
            object: None,
        }
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
    fn reflection(&self) -> f32 {
        self.reflection
    }
    fn transparency(&self) -> f32 {
        self.transparency
    }
    fn amb_texture(&self) -> Option<Arc<Texture>> {
        None
    }
    fn diff_texture(&self) -> Option<Arc<Texture>> {
        None
    }
}

// 삼각형(Triangle)을 나타내는 구조체
// C++: Triangle 클래스
struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    uv0: Vec2,
    uv1: Vec2,
    uv2: Vec2,
}

impl Triangle {
    // 광선과 삼각형 교점 찾기 (Möller–Trumbore intersection algorithm과 유사한 기하학적 해법)
    // C++: IntersectRayTriangle 함수
    fn intersect_ray_triangle(
        &self,
        ray: &Ray,
    ) -> Option<(f32, Vec3, Vec3, f32, f32)> {
        let face_normal =
            (self.v1 - self.v0).cross(self.v2 - self.v0).normalize();

        // Backface culling: 삼각형 뒷면은 그리지 않음
        if (-ray.direction).dot(face_normal) < 0.0 {
            return None;
        }

        // 평면과 광선이 거의 평행하면 충돌하지 않음
        if ray.direction.dot(face_normal).abs() < 1e-2 {
            return None;
        }

        // 광선과 평면의 교점 계산
        let t = (self.v0.dot(face_normal)
            - ray.origin.dot(face_normal))
            / ray.direction.dot(face_normal);

        if t < 0.0 {
            return None;
        }

        let point = ray.origin + t * ray.direction;

        // 교점이 삼각형 내부에 있는지 확인 (Inside-Outside Test)
        let cross0 = (point - self.v2).cross(self.v1 - self.v2);
        let cross1 = (point - self.v0).cross(self.v2 - self.v0);
        let cross2 = (self.v1 - self.v0).cross(point - self.v0);

        if cross0.dot(face_normal) < 0.0
            || cross1.dot(face_normal) < 0.0
            || cross2.dot(face_normal) < 0.0
        {
            return None;
        }

        // 무게중심 좌표(Barycentric coordinates) 계산. 텍스처 좌표 보간에 사용
        let area0 = cross0.length() * 0.5;
        let area1 = cross1.length() * 0.5;
        let area_sum =
            (self.v1 - self.v0).cross(self.v2 - self.v0).length()
                * 0.5;

        let w0 = area0 / area_sum;
        let w1 = area1 / area_sum;

        Some((t, point, face_normal, w0, w1))
    }
}

// 사각형(Square)을 나타내는 구조체
// C++: Square 클래스
struct Square {
    triangle1: Triangle,
    triangle2: Triangle,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
    reflection: f32,
    transparency: f32,
    amb_texture: Option<Arc<Texture>>,
    diff_texture: Option<Arc<Texture>>,
}

impl Object for Square {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        let hit1 = self.triangle1.intersect_ray_triangle(ray);
        let hit2 = self.triangle2.intersect_ray_triangle(ray);

        let best_hit = match (hit1, hit2) {
            (Some(h1), Some(h2)) => {
                if h1.0 < h2.0 {
                    Some((h1, &self.triangle1))
                } else {
                    Some((h2, &self.triangle2))
                }
            }
            (Some(h1), None) => Some((h1, &self.triangle1)),
            (None, Some(h2)) => Some((h2, &self.triangle2)),
            (None, None) => None,
        };

        if let Some(((t, point, normal, w0, w1), tri)) = best_hit {
            let w2 = 1.0 - w0 - w1;
            let uv = tri.uv0 * w0 + tri.uv1 * w1 + tri.uv2 * w2;
            Hit {
                distance: t,
                point,
                normal,
                uv,
                object: None,
            }
        } else {
            Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                uv: Vec2::ZERO,
                object: None,
            }
        }
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
    fn reflection(&self) -> f32 {
        self.reflection
    }
    fn transparency(&self) -> f32 {
        self.transparency
    }
    fn amb_texture(&self) -> Option<Arc<Texture>> {
        self.amb_texture.clone()
    }
    fn diff_texture(&self) -> Option<Arc<Texture>> {
        self.diff_texture.clone()
    }
}

// 점 광원(Point Light)을 나타내는 구조체
// C++: Light 클래스
struct Light {
    pos: Vec3,
}

// 텍스처 데이터를 담는 구조체
// C++: Texture 클래스
struct Texture {
    width: u32,
    height: u32,
    channels: u32,
    image: Vec<u8>,
}

impl Texture {
    fn from_dynamic_image(img: image::DynamicImage) -> Self {
        let (width, height) = img.dimensions();
        let image_data = img.to_rgba8().into_raw();
        Self {
            width,
            height,
            channels: 4,
            image: image_data,
        }
    }

    // C++: GetWrapped 함수
    fn get_wrapped(&self, mut i: i32, mut j: i32) -> Vec3 {
        i %= self.width as i32;
        j %= self.height as i32;
        if i < 0 {
            i += self.width as i32;
        }
        if j < 0 {
            j += self.height as i32;
        }

        let idx = ((j as u32 * self.width + i as u32) * self.channels)
            as usize;
        let r = self.image[idx] as f32 / 255.0;
        let g = self.image[idx + 1] as f32 / 255.0;
        let b = self.image[idx + 2] as f32 / 255.0;
        vec3(r, g, b)
    }

    // C++: InterpolateBilinear 함수
    fn interpolate_bilinear(
        &self,
        dx: f32,
        dy: f32,
        c00: Vec3,
        c10: Vec3,
        c01: Vec3,
        c11: Vec3,
    ) -> Vec3 {
        let a = c00.lerp(c10, dx);
        let b = c01.lerp(c11, dx);
        a.lerp(b, dy)
    }

    // C++: SampleLinear 함수
    fn sample_linear(&self, uv: Vec2) -> Vec3 {
        let xy = uv * vec2(self.width as f32, self.height as f32)
            - vec2(0.5, 0.5);
        let i = xy.x.floor() as i32;
        let j = xy.y.floor() as i32;
        let dx = xy.x - i as f32;
        let dy = xy.y - j as f32;

        let c00 = self.get_wrapped(i, j);
        let c10 = self.get_wrapped(i + 1, j);
        let c01 = self.get_wrapped(i, j + 1);
        let c11 = self.get_wrapped(i + 1, j + 1);

        self.interpolate_bilinear(dx, dy, c00, c10, c01, c11)
    }
}

// 레이 트레이서
// C++: Raytracer 클래스
struct Raytracer {
    width: i32,
    height: i32,
    objects: Vec<Arc<dyn Object>>,
    light: Light,
}

impl Raytracer {
    fn new(width: i32, height: i32) -> Self {
        let sphere1 = Arc::new(Sphere {
            center: vec3(0.0, -0.1, 1.5),
            radius: 1.0,
            amb: vec3(0.2, 0.2, 0.2),
            diff: vec3(0.0, 0.0, 1.0),
            spec: Vec3::ZERO,
            alpha: 50.0,
            reflection: 0.0,
            transparency: 1.0, // 굴절을 위해 투명하게 설정
        });

        let ground_texture_bytes =
            include_bytes!("../assets/shadertoy_abstract1.jpg");
        let ground_image =
            image::load_from_memory(ground_texture_bytes).unwrap();
        let ground_texture =
            Arc::new(Texture::from_dynamic_image(ground_image));

        let ground = Arc::new(Square {
            triangle1: Triangle {
                v0: vec3(-10.0, -1.5, 0.0),
                v1: vec3(-10.0, -1.5, 10.0),
                v2: vec3(10.0, -1.5, 10.0),
                uv0: vec2(0.0, 0.0),
                uv1: vec2(1.0, 0.0),
                uv2: vec2(1.0, 1.0),
            },
            triangle2: Triangle {
                v0: vec3(-10.0, -1.5, 0.0),
                v1: vec3(10.0, -1.5, 10.0),
                v2: vec3(10.0, -1.5, 0.0),
                uv0: vec2(0.0, 0.0),
                uv1: vec2(1.0, 1.0),
                uv2: vec2(0.0, 1.0),
            },
            amb: vec3(1.0, 1.0, 1.0),
            diff: vec3(1.0, 1.0, 1.0),
            spec: vec3(1.0, 1.0, 1.0),
            alpha: 10.0,
            reflection: 0.0,
            transparency: 0.0,
            amb_texture: Some(ground_texture.clone()),
            diff_texture: Some(ground_texture),
        });

        let back_texture_bytes = include_bytes!("../assets/back.jpg");
        let back_image =
            image::load_from_memory(back_texture_bytes).unwrap();
        let back_texture =
            Arc::new(Texture::from_dynamic_image(back_image));

        let back_square = Arc::new(Square {
            triangle1: Triangle {
                v0: vec3(-10.0, 10.0, 10.0),
                v1: vec3(10.0, 10.0, 10.0),
                v2: vec3(10.0, -10.0, 10.0),
                uv0: vec2(0.0, 0.0),
                uv1: vec2(1.0, 0.0),
                uv2: vec2(1.0, 1.0),
            },
            triangle2: Triangle {
                v0: vec3(-10.0, 10.0, 10.0),
                v1: vec3(10.0, -10.0, 10.0),
                v2: vec3(-10.0, -10.0, 10.0),
                uv0: vec2(0.0, 0.0),
                uv1: vec2(1.0, 1.0),
                uv2: vec2(0.0, 1.0),
            },
            amb: vec3(1.0, 1.0, 1.0),
            diff: vec3(0.0, 0.0, 0.0),
            spec: Vec3::ZERO,
            alpha: 10.0,
            reflection: 0.0,
            transparency: 0.0,
            amb_texture: Some(back_texture.clone()),
            diff_texture: Some(back_texture),
        });

        let objects: Vec<Arc<dyn Object>> =
            vec![sphere1, ground, back_square];

        Self {
            width,
            height,
            objects,
            light: Light {
                pos: vec3(0.0, 0.3, -0.5),
            },
        }
    }

    // C++: FindClosestCollision 함수
    fn find_closest_collision(&self, ray: &Ray) -> Hit {
        let mut closest_d = f32::MAX;
        let mut closest_hit = Hit {
            distance: -1.0,
            point: Vec3::ZERO,
            normal: Vec3::ZERO,
            uv: Vec2::ZERO,
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

    // C++: traceRay 함수
    fn trace_ray(&self, ray: &Ray, recurse_level: i32) -> Vec3 {
        if recurse_level < 0 {
            return Vec3::ZERO;
        }

        let hit = self.find_closest_collision(ray);

        if let Some(obj) = hit.object {
            let mut color = Vec3::ZERO;
            let dir_to_light =
                (self.light.pos - hit.point).normalize();
            let mut phong_color = Vec3::ZERO;
            let diff = hit.normal.dot(dir_to_light).max(0.0);
            let reflect_dir =
                2.0 * hit.normal * hit.normal.dot(dir_to_light)
                    - dir_to_light;
            let specular = (-ray.direction)
                .dot(reflect_dir)
                .max(0.0)
                .powf(obj.alpha());

            if let Some(tex) = obj.amb_texture() {
                phong_color +=
                    obj.ambient() * tex.sample_linear(hit.uv);
            } else {
                phong_color += obj.ambient();
            }

            if let Some(tex) = obj.diff_texture() {
                phong_color +=
                    diff * obj.diffuse() * tex.sample_linear(hit.uv);
            } else {
                phong_color += diff * obj.diffuse();
            }

            phong_color += obj.specular() * specular;
            color += phong_color
                * (1.0 - obj.reflection() - obj.transparency());

            if obj.reflection() > 0.0 {
                let reflected_direction = (ray.direction
                    - 2.0
                        * ray.direction.dot(hit.normal)
                        * hit.normal)
                    .normalize();
                let reflection_ray = Ray {
                    origin: hit.point + reflected_direction * 1e-4,
                    direction: reflected_direction,
                };
                color += self
                    .trace_ray(&reflection_ray, recurse_level - 1)
                    * obj.reflection();
            }

            // ==================================================================
            // 여기가 핵심! 굴절(Refraction) 구현 부분입니다.
            // ==================================================================
            if obj.transparency() > 0.0 {
                /*
                 * ### 굴절(Refraction) 원리 (스넬의 법칙, Snell's Law)
                 * 빛이 서로 다른 매질(공기, 물, 유리 등)의 경계를 지날 때 꺾이는 현상입니다.
                 * 물리 법칙에 따라, 입사각(theta1)과 굴절각(theta2)의 사인(sin) 값의 비율은
                 * 두 매질의 굴절률(Index of Refraction, IOR) 비율과 같습니다.
                 *
                 * sin(theta1) / sin(theta2) = n2 / n1 = eta (η)
                 *
                 * 여기서 n1은 현재 매질의 굴절률, n2는 다음 매질의 굴절률입니다.
                 * 우리는 이 공식을 사용해 굴절된 빛의 방향 벡터를 계산할 것입니다.
                 */
                const IOR: f32 = 1.5; // 굴절률 (유리)

                /*
                 * ### 바깥에서의 충돌과 안에서의 충돌
                 * 광선이 물체 외부에서 내부로 들어가는지(공기->유리),
                 * 또는 내부에서 외부로 나가는지(유리->공기)를 판단해야 합니다.
                 *
                 * - ray.direction.dot(hit.normal) < 0.0 : 광선이 표면의 앞면과 충돌 (밖 -> 안) /
                 * - ray.direction.dot(hit.normal) > 0.0 : 광선이 표면의 뒷면과 충돌 (안 -> 밖) /
                 *
                 * C++ 예제에서는 구(Sphere)의 충돌 판정에서 두 개의 교점(d1, d2)을 모두 계산하고,
                 * 광선이 구 내부에서 시작된 경우 더 먼 교점(d2)을 선택하는 방식으로 이를 처리했습니다.
                 * 여기서는 더 직관적으로 법선 벡터(normal)와 에타(eta) 값을 조정하는 방식을 사용합니다.
                 */
                let (eta, normal) =
                    if ray.direction.dot(hit.normal) < 0.0 {
                        // 광선이 밖에서 안으로 들어가는 경우 (예: 공기 -> 유리)
                        (1.0 / IOR, hit.normal) // eta = n1(공기)/n2(유리) ~= 1.0 / 1.5
                    } else {
                        // 광선이 안에서 밖으로 나가는 경우 (예: 유리 -> 공기)
                        (IOR, -hit.normal) // eta = n1(유리)/n2(공기) ~= 1.5 / 1.0, 법선 벡터를 뒤집어 광선과 같은 방향을 보게 함
                    };

                /*
                 * ### 굴절 벡터 계산 유도
                 * 굴절된 방향 벡터 t를 구하기 위해 삼각함수를 사용합니다.
                 * d: 입사 방향 벡터, n: 법선 벡터, t: 굴절 방향 벡터
                 *
                 * 1. cos(theta1) 계산:
                 *    cos(theta1) = dot(-d, n)
                 *
                 * 2. sin(theta1) 계산:
                 *    sin^2(theta) = 1 - cos^2(theta) 를 이용
                 *
                 * 3. sin(theta2) 계산:
                 *    스넬의 법칙: sin(theta2) = sin(theta1) * eta
                 *
                 * 4. cos(theta2) 계산:
                 *    sin^2(theta2) + cos^2(theta2) = 1 이용
                 *
                 * 5. 굴절 벡터 t 계산:
                 *    t = n * (-cos(theta2)) + m * sin(theta2)
                 *    여기서 m은 법선에 수직인 접선 방향 벡터입니다.
                 *    m = normalize(d + n * cos(theta1))
                 *    이 식들을 조합하고 정리하면 아래와 같은 최종 굴절 벡터 공식을 얻을 수 있습니다.
                 *    t = eta * d + (eta * cos(theta1) - cos(theta2)) * n
                 */
                let cos_theta1 = (-ray.direction).dot(normal);
                let sin2_theta1 = 1.0 - cos_theta1 * cos_theta1;
                let sin2_theta2 = sin2_theta1 * eta * eta;

                // 전반사(Total Internal Reflection) 체크
                // sin(theta2)가 1보다 크면 빛이 굴절하지 않고 모두 반사됩니다.
                if sin2_theta2 < 1.0 {
                    let cos_theta2 = (1.0 - sin2_theta2).sqrt();
                    let refracted_direction = (ray.direction * eta
                        + normal * (eta * cos_theta1 - cos_theta2))
                        .normalize();

                    // 부동소수점 오류를 피하기 위해 살짝 떨어진 위치에서 새로운 광선 시작
                    let refraction_ray = Ray {
                        origin: hit.point
                            + refracted_direction * 1e-4,
                        direction: refracted_direction,
                    };
                    // 재귀적으로 굴절된 광선을 추적하고, 투명도를 곱해 색상에 더함
                    color += self.trace_ray(
                        &refraction_ray,
                        recurse_level - 1,
                    ) * obj.transparency();
                } else {
                    // 전반사가 일어날 경우, 빛은 모두 반사됨 (Fresnel 효과를 단순화)
                    let reflected_direction = (ray.direction
                        - 2.0
                            * ray.direction.dot(hit.normal)
                            * hit.normal)
                        .normalize();
                    let reflection_ray = Ray {
                        origin: hit.point
                            + reflected_direction * 1e-4,
                        direction: reflected_direction,
                    };
                    color += self.trace_ray(
                        &reflection_ray,
                        recurse_level - 1,
                    ) * obj.transparency();
                }
            }
            color
        } else {
            Vec3::ZERO
        }
    }

    // C++: TransformScreenToWorld 함수
    fn transform_screen_to_world(&self, pos_screen: Vec2) -> Vec3 {
        let x_scale = 2.0 / self.width as f32;
        let y_scale = 2.0 / self.height as f32;
        let aspect = self.width as f32 / self.height as f32;

        vec3(
            (pos_screen.x * x_scale - 1.0) * aspect,
            -pos_screen.y * y_scale + 1.0,
            0.0,
        )
    }

    // C++: Render 함수
    fn render(&mut self, pixels: &mut [Color32]) {
        let eye_pos = vec3(0.0, 0.0, -1.5);

        pixels.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
            let i = idx % self.width as usize;
            let j = idx / self.width as usize;

            let pos_world = self
                .transform_screen_to_world(vec2(i as f32, j as f32));
            let ray_dir = (pos_world - eye_pos).normalize();
            let pixel_ray = Ray {
                origin: pos_world,
                direction: ray_dir,
            };
            let color_vec = self.trace_ray(&pixel_ray, 5);

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
    #[serde(skip)]
    is_first_frame: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            raytracer: Raytracer::new(1280, 720),
            is_first_frame: true,
        }
    }
}

impl TemplateApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let width = self.raytracer.width as usize;
            let height = self.raytracer.height as usize;

            // C++의 `if (count == 0)` 와 같이 첫 프레임에만 렌더링
            if self.is_first_frame {
                let mut pixels: Vec<Color32> =
                    vec![Color32::BLACK; width * height];
                self.raytracer.render(&mut pixels);

                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width, height],
                    bytemuck::cast_slice(&pixels),
                );

                // 렌더링된 이미지를 egui 컨텍스트 메모리에 저장
                ctx.memory_mut(|mem| {
                    mem.data.insert_temp(
                        egui::Id::new("raytrace_texture"),
                        image,
                    )
                });

                self.is_first_frame = false;
            }

            // 메모리에 저장된 이미지를 불러와서 화면에 표시
            if let Some(image) = ctx.memory(|mem| {
                mem.data.get_temp::<egui::ColorImage>(egui::Id::new(
                    "raytrace_texture",
                ))
            }) {
                let texture_handle = ctx.load_texture(
                    "raytrace_canvas",
                    image.clone(),
                    TextureOptions::NEAREST,
                );
                let img_widget = egui::Image::new(&texture_handle)
                    .fit_to_original_size(1.0)
                    .shrink_to_fit();
                ui.centered_and_justified(|ui| {
                    ui.add(img_widget);
                });
            }
        });
    }
}
