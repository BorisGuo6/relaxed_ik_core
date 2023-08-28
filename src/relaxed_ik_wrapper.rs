use crate::relaxed_ik::{RelaxedIK, Opt};
use std::sync::{Arc, Mutex};
use nalgebra::{Vector3, Vector6, UnitQuaternion, Quaternion,Translation3, Isometry3};
use std::os::raw::{*};
use std::str;
use std::os::raw::{c_char, c_void};
use std::ffi::CString;
use std::ptr;
use crate::utils_rust::file_utils::{*};
use std::time::Instant;

// http://jakegoulding.com/rust-ffi-omnibus/objects/
#[no_mangle]
pub unsafe extern "C" fn relaxed_ik_new(path_to_setting: *const c_char) -> *mut RelaxedIK {
    if path_to_setting.is_null() 
    { 
        let path_to_src = get_path_to_src();
        let default_path_to_setting = path_to_src +  "configs/settings.yaml";
        return Box::into_raw(Box::new(RelaxedIK::load_settings(default_path_to_setting.as_str())))
    }
    let c_str = std::ffi::CStr::from_ptr(path_to_setting);
    let path_to_setting_str = c_str.to_str().expect("Not a valid UTF-8 string");

    Box::into_raw(Box::new(RelaxedIK::load_settings(path_to_setting_str)))
}

#[no_mangle] 
pub unsafe extern "C" fn relaxed_ik_free(ptr: *mut RelaxedIK) {
    if ptr.is_null() { return }
    Box::from_raw(ptr);
}

#[no_mangle]
pub unsafe extern "C" fn reset(ptr: *mut RelaxedIK, joint_state: *const c_double, joint_state_length: c_int) {
    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let x_slice: &[c_double] = std::slice::from_raw_parts(joint_state, joint_state_length as usize);
    let x_vec = x_slice.to_vec();
    relaxed_ik.reset(x_vec);
}

#[no_mangle]
pub unsafe extern "C" fn get_jointstate_loss(ptr: *mut RelaxedIK, joint_state: *const c_double, joint_state_length: c_int) -> Opt {
    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let x_slice: &[c_double] = std::slice::from_raw_parts(joint_state, joint_state_length as usize);
    let x_vec = x_slice.to_vec();
    let losses = relaxed_ik.get_jointstate_loss(x_vec);

    let ptr = losses.as_ptr();
    let len = losses.len() as c_int;
    std::mem::forget(losses);
    Opt { data: ptr, length: len }
}

#[no_mangle]
pub unsafe extern "C" fn solve_position(ptr: *mut RelaxedIK, pos_goals: *const c_double, pos_length: c_int, 
    quat_goals: *const c_double, quat_length: c_int,
    tolerance: *const c_double, tolerance_length: c_int) -> Opt {

    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    
    assert!(!pos_goals.is_null(), "Null pointer for pos goals!");
    assert!(!quat_goals.is_null(), "Null pointer for quat goals!");
    assert!(!tolerance.is_null(), "Null pointer for tolerance!"); 

    let num_active_chains: usize = relaxed_ik.vars.is_active_chain.iter().map(|x| {*x as usize}).sum();
    assert!(pos_length as usize == num_active_chains * 3 , 
        "Pos vels are expected to have {} numbers, but got {}", 
        num_active_chains * 3, pos_length);
    assert!(quat_length as usize == num_active_chains * 4,
        "Rot vels are expected to have {} numbers, but got {}", 
        num_active_chains * 4, quat_length);
    assert!(tolerance_length as usize == num_active_chains * 6, 
        "Tolerance are expected to have {} numbers, but got {}", 
        num_active_chains * 6, tolerance_length);
    
    // let start = Instant::now();
    
    let pos_slice: &[c_double] = std::slice::from_raw_parts(pos_goals, pos_length as usize);
    let quat_slice: &[c_double] = std::slice::from_raw_parts(quat_goals, quat_length as usize);
    let tolerance_slice: &[c_double] = std::slice::from_raw_parts(tolerance, tolerance_length as usize);

    let pos_vec = pos_slice.to_vec();
    let quat_vec = quat_slice.to_vec();
    let tolerance_vec = tolerance_slice.to_vec();

    let ja = solve_position_helper(relaxed_ik, pos_vec, quat_vec, tolerance_vec);

    let ptr = ja.as_ptr();
    let len = ja.len();
    std::mem::forget(ja);

    // let elapsed = start.elapsed();
    // println!("Rust time: {:?} ms", elapsed.as_millis());


    Opt {data: ptr, length: len as c_int}
}


#[no_mangle]
pub unsafe extern "C" fn solve_velocity(ptr: *mut RelaxedIK, pos_vels: *const c_double, pos_length: c_int, 
    rot_vels: *const c_double, rot_length: c_int,
    tolerance: *const c_double, tolerance_length: c_int) -> Opt {

    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    
    assert!(!pos_vels.is_null(), "Null pointer for pos vels!");
    assert!(!rot_vels.is_null(), "Null pointer for rot vels!");
    assert!(!tolerance.is_null(), "Null pointer for tolerance!"); 
    let num_active_chains: usize = relaxed_ik.vars.is_active_chain.iter().map(|x| {*x as usize}).sum();
    assert!(pos_length as usize == num_active_chains * 3 , 
        "Pos vels are expected to have {} numbers, but got {}", 
        num_active_chains * 3, pos_length);
    assert!(rot_length as usize == num_active_chains * 3,
        "Rot vels are expected to have {} numbers, but got {}", 
        num_active_chains * 3, rot_length);
    assert!(tolerance_length as usize == num_active_chains * 6, 
        "Tolerance are expected to have {} numbers, but got {}", 
        num_active_chains * 6, tolerance_length);

    let pos_slice: &[c_double] = std::slice::from_raw_parts(pos_vels, pos_length as usize);
    let rot_slice: &[c_double] = std::slice::from_raw_parts(rot_vels, rot_length as usize);
    let tolerance_slice: &[c_double] = std::slice::from_raw_parts(tolerance, tolerance_length as usize);

    let pos_vec = pos_slice.to_vec();
    let rot_vec = rot_slice.to_vec();
    let tolerance_vec = tolerance_slice.to_vec();

    let ja = solve_velocity_helper(relaxed_ik, pos_vec, rot_vec, tolerance_vec);

    let ptr = ja.as_ptr();
    let len = ja.len();
    std::mem::forget(ja);
    Opt {data: ptr, length: len as c_int}
}

#[no_mangle]
pub unsafe extern "C" fn get_ee_positions(ptr: *mut RelaxedIK) -> Opt {
    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let mut positions = Vec::new();
    for i in 0..relaxed_ik.vars.goal_positions.len() {
        positions.push(relaxed_ik.vars.goal_positions[i].x);
        positions.push(relaxed_ik.vars.goal_positions[i].y);
        positions.push(relaxed_ik.vars.goal_positions[i].z);
    }
    let ptr = positions.as_ptr();
    let len = positions.len();
    std::mem::forget(positions);
    Opt {data: ptr, length: len as c_int}
}

// This is mainly for backward compatibility
#[no_mangle]
pub unsafe extern "C" fn solve(ptr: *mut RelaxedIK, pos_goals: *const c_double, pos_length: c_int, 
    quat_goals: *const c_double, quat_length: c_int,
    tolerance: *const c_double, tolerance_length: c_int) -> Opt {

    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    
    assert!(!pos_goals.is_null(), "Null pointer for pos goals!");
    assert!(!quat_goals.is_null(), "Null pointer for quat goals!");
    assert!(!tolerance.is_null(), "Null pointer for tolerance!"); 

    let pos_slice: &[c_double] = std::slice::from_raw_parts(pos_goals, pos_length as usize);
    let quat_slice: &[c_double] = std::slice::from_raw_parts(quat_goals, quat_length as usize);
    let tolerance_slice: &[c_double] = std::slice::from_raw_parts(tolerance, tolerance_length as usize);

    let pos_vec = pos_slice.to_vec();
    let quat_vec = quat_slice.to_vec();
    let tolerance_vec = tolerance_slice.to_vec();
    let ja = solve_position_helper(relaxed_ik, pos_vec, quat_vec, tolerance_vec);

    let ptr = ja.as_ptr();
    let len = ja.len();
    std::mem::forget(ja);
    Opt {data: ptr, length: len as c_int}
}

fn solve_position_helper(relaxed_ik: &mut RelaxedIK, pos_goals: Vec<f64>, quat_goals: Vec<f64>,
                tolerance: Vec<f64>) -> Vec<f64> {
    let mut j: usize = 0;
    for i in 0..relaxed_ik.vars.robot.num_chains  {
        if relaxed_ik.vars.is_active_chain[i] {
            relaxed_ik.vars.goal_positions[i] = Vector3::new(pos_goals[3*j], pos_goals[3*j+1], pos_goals[3*j+2]);
            let tmp_q = Quaternion::new(quat_goals[4*j+3], quat_goals[4*j], quat_goals[4*j+1], quat_goals[4*j+2]);
            relaxed_ik.vars.goal_quats[i] =  UnitQuaternion::from_quaternion(tmp_q);
            relaxed_ik.vars.tolerances[i] = Vector6::new( tolerance[6*j], tolerance[6*j+1], tolerance[6*j+2],
            tolerance[6*j+3], tolerance[6*j+4], tolerance[6*j+5]);

            j += 1;
        }
    }
                    
    let x = relaxed_ik.solve();
    return x;
}

fn solve_velocity_helper(relaxed_ik: &mut RelaxedIK, pos_vels: Vec<f64>, rot_vels: Vec<f64>,
    tolerance: Vec<f64>) -> Vec<f64> {

    let mut j: usize = 0;
    for i in 0..relaxed_ik.vars.robot.num_chains  {
        if relaxed_ik.vars.is_active_chain[i] {
            relaxed_ik.vars.goal_positions[i] += Vector3::new(pos_vels[3*j], pos_vels[3*j+1], pos_vels[3*j+2]);
            let axisangle = Vector3::new(rot_vels[3*j], rot_vels[3*j+1], rot_vels[3*j+2]);
            let tmp_q = UnitQuaternion::from_scaled_axis(axisangle);
            let org_q = relaxed_ik.vars.goal_quats[i].clone();
            relaxed_ik.vars.goal_quats[i] =  tmp_q * org_q;
            relaxed_ik.vars.tolerances[i] = Vector6::new( tolerance[3*j], tolerance[3*j+1], tolerance[3*j+2],
                tolerance[3*j+3], tolerance[3*j+4], tolerance[3*j+5]);

            j += 1;
        }
    }

    let x = relaxed_ik.solve();


    let frames = relaxed_ik.vars.robot.get_frames_immutable(&x);
    let last = frames[0].0.len() - 1 ;
    let ee_pos = frames[0].0[last];
    let goal = relaxed_ik.vars.goal_positions[0];
    let dist = (ee_pos - goal).norm();

    return x;
}

#[no_mangle]
pub unsafe extern "C" fn dynamic_obstacle_cb(ptr: *mut RelaxedIK, name: *const c_char, pos_arr: *const c_double, quat_arr: *const c_double) {
    assert!(!name.is_null(), "Empty name!");
    assert!(!pos_arr.is_null(), "Null pointer for pos!");
    assert!(!quat_arr.is_null(), "Null pointer for quat!");

    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let c_str = std::ffi::CStr::from_ptr(name);
    let name_str = c_str.to_str().expect("Not a valid UTF-8 string");

    let pos_slice: &[c_double] = std::slice::from_raw_parts(pos_arr, 3);
    let quat_slice: &[c_double] = std::slice::from_raw_parts(quat_arr, 4);

    let pos_vec = pos_slice.to_vec();
    let quat_vec = quat_slice.to_vec();

    let ts = Translation3::new(pos_vec[0], pos_vec[1], pos_vec[2]);
    let tmp_q = Quaternion::new(quat_vec[3], quat_vec[0], quat_vec[1], quat_vec[2]);
    let rot = UnitQuaternion::from_quaternion(tmp_q);
    let pos = Isometry3::from_parts(ts, rot);

    // println!("dynamic_obstacle_cb: {:?} -> {:?}",name_str, pos);
    relaxed_ik.vars.env_collision.update_dynamic_obstacle(name_str, pos);
}


#[no_mangle]
pub unsafe extern "C" fn get_objective_weight_priors(ptr: *mut RelaxedIK) -> Opt {
    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let ptr = relaxed_ik.om.weight_priors.as_ptr();
    let len = relaxed_ik.om.weight_priors.len();
    std::mem::forget(ptr);
    Opt {data: ptr, length: len as c_int}
}

#[no_mangle]
pub unsafe extern "C" fn set_objective_weight_priors(ptr: *mut RelaxedIK, weights: *const c_double, weights_len: c_int) {
    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    assert!(!weights.is_null(),"Null pointer for weights!");
    assert!(weights_len as usize == relaxed_ik.om.weight_priors.len());

    let w = std::slice::from_raw_parts(weights, weights_len as usize);
    for i in 0..relaxed_ik.om.weight_priors.len() {
        relaxed_ik.om.weight_priors[i] = w[i];
    }
}

#[no_mangle]
pub unsafe extern "C" fn set_env_collision_tip_offset(ptr: *mut RelaxedIK, offset: c_int) {
    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };
    assert!(offset as i64 >= 0, "Disabled collision offset from tip should be non-negative.");

    relaxed_ik.vars.env_collision_tip_offset = offset as usize;
    relaxed_ik.om.set_env_collision_tip_offset(offset as usize);
}

#[repr(C)]
pub struct StringArray {
    data: *mut *mut c_char,
    len: usize,
}

#[no_mangle]
pub unsafe extern "C" fn get_objective_weight_names(ptr: *mut RelaxedIK) -> *mut c_void {
    let relaxed_ik = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };


    let mut c_strings = Vec::new();

    for s in &relaxed_ik.om.weight_names {
        let c_string = CString::new(s.clone()).unwrap();
        c_strings.push(c_string.into_raw());
    }

    let string_array = StringArray {
        data: c_strings.as_mut_ptr(),
        len: c_strings.len(),
    };

    // Prevent the destructor from being called, as the memory is now owned
    // by Python code.
    let _ = Box::into_raw(c_strings.into_boxed_slice());

    Box::into_raw(Box::new(string_array)) as *mut c_void
}