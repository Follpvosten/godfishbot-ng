use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use tbot::contexts::{methods::Message, Command};

use crate::ResultExt;

pub static USAGE: &str = "/testlove <list of names>";
pub async fn handler(ctx: Arc<Command>) {
    if ctx.text.value.is_empty() {
        ctx.send_message_in_reply(USAGE).call().await.log_err();
        return;
    }
    let names: Vec<&str> = if ctx.text.value.contains('\n') {
        ctx.text.value.split('\n').collect()
    } else {
        ctx.text.value.split(' ').collect()
    };
    if names.len() < 2 {
        ctx.send_message_in_reply("Please provide at least two names.")
            .call()
            .await
            .log_err();
        return;
    }
    let result = if names.len() > 2 {
        rank_love(&names)
    } else {
        let str_result = test_love(names[0], names[1]);
        format!("{} and {} fit {}%.", names[0], names[1], str_result)
    };
    ctx.send_message(result).call().await.log_err();
}

fn get_count(name1: &str, name2: &str) -> Vec<usize> {
    const LOVE_VAL: &str = "ILOVE";
    let mut map = BTreeMap::new();
    let names = name1.to_ascii_uppercase() + &name2.to_ascii_uppercase() + LOVE_VAL;
    for ch in names.chars() {
        if let Some(val) = map.get_mut(&ch) {
            *val += 1;
        } else {
            map.insert(ch, 1);
        }
    }
    map.values().copied().collect()
}

fn test_love(name1: &str, name2: &str) -> String {
    let (name1, name2) = if name1 > name2 {
        (name1, name2)
    } else {
        (name2, name1)
    };
    let mut count = get_count(name1, name2);
    if count.len() == 1 {
        return count[0].to_string();
    } else if count.len() == 2 {
        return count[0].to_string() + &count[1].to_string();
    }
    while count.len() != 2 {
        let mut sub: Vec<usize> = Vec::new();
        let size = count.len() / 2;
        for i in 0..size {
            let new_c = (count[i] + count[count.len() - 1 - i]).to_string();
            new_c
                .chars()
                .filter_map(|c| c.to_string().parse().ok())
                .for_each(|c| sub.push(c));
        }
        if count.len() != size * 2 {
            sub.push(count[size]);
        }
        count = sub;
    }
    count[0].to_string() + &count[1].to_string()
}

fn rank_love(names: &[&str]) -> String {
    let mut combos = HashMap::new();
    for name1 in names.iter() {
        for name2 in names.iter() {
            if combos.contains_key(&(*name1, *name2))
                || combos.contains_key(&(*name2, *name1))
                || *name1 == *name2
            {
                continue;
            }
            combos.insert((*name1, *name2), test_love(name1, name2));
        }
    }
    let mut tmp = combos.iter().collect::<Vec<_>>();
    tmp.sort_by(|t1, t2| t2.1.partial_cmp(t1.1).unwrap());
    tmp.iter()
        .enumerate()
        .map(|(i, ((name1, name2), result))| {
            format!("{}. {} x {} ({}%)", i + 1, name1, name2, result)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
